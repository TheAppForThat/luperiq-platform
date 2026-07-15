//! IQtags — the pure, I/O-free smart-tag core.
//!
//! An **IQtag is a graph edge** from a piece of content (a comment, a profile, a
//! block) to a target knowledge node. "Smart" means: as you type — or as a comment
//! is posted — text resolves to the *right* node. This core does the three pure
//! parts of that: [`parse`] tag tokens out of text (`@mention`, `#hashtag`,
//! `[[wikilink]]`), [`normalize`] a string to a slug, and [`rank`] candidate nodes
//! for a query. The *fast* candidate lookup (FTS5 prefix + slug/alias index) is the
//! store adapter's job; this core only ranks what the adapter hands it, so it stays
//! dependency-free and exhaustively testable.

use serde::{Deserialize, Serialize};

/// The sigil a tag token used in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    /// `@german-cockroach`
    Mention,
    /// `#rodents`
    Hashtag,
    /// `[[Norway Rat]]`
    Wikilink,
}

/// A tag token found in text, with byte span for highlight/replace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub kind: TagKind,
    /// The text after the sigil ("german-cockroach", "Norway Rat").
    pub raw: String,
    pub start: usize,
    pub end: usize,
}

/// A candidate node the store adapter found cheaply (one row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub pack: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub node_type: String,
    /// Tie-breaker (e.g. backlink count). 0 if unknown.
    #[serde(default)]
    pub popularity: i64,
}

/// A scored candidate (higher = better), with why it matched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scored {
    pub candidate: Candidate,
    pub score: i32,
    pub reason: &'static str,
}

/// Normalize a string to a slug — mirrors the store's `slugify` so a normalized
/// query compares directly against stored slugs (lowercase ascii, `-` separators).
pub fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[inline]
fn utf8_len(b: u8) -> usize {
    if b < 0x80 { 1 } else if b < 0xE0 { 2 } else if b < 0xF0 { 3 } else { 4 }
}

/// Parse `@mention`, `#hashtag`, and `[[wikilink]]` tokens out of free text.
pub fn parse(text: &str) -> Vec<Token> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'@' || c == b'#' {
            let kind = if c == b'@' { TagKind::Mention } else { TagKind::Hashtag };
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            if j > i + 1 {
                out.push(Token { kind, raw: text[i + 1..j].to_string(), start: i, end: j });
                i = j;
                continue;
            }
        } else if c == b'[' && i + 1 < b.len() && b[i + 1] == b'[' {
            if let Some(rel) = text[i + 2..].find("]]") {
                let inner = &text[i + 2..i + 2 + rel];
                let raw = inner.trim().to_string();
                let end = i + 2 + rel + 2;
                if !raw.is_empty() {
                    out.push(Token { kind: TagKind::Wikilink, raw, start: i, end });
                }
                i = end;
                continue;
            }
        }
        i += utf8_len(c);
    }
    out
}

/// Rank candidates for a query: exact slug/name > alias > prefix > contains, with
/// popularity as a tie-breaker. Non-matches are dropped. Empty query → empty.
pub fn rank(query: &str, candidates: &[Candidate]) -> Vec<Scored> {
    let qn = normalize(query);
    if qn.is_empty() {
        return Vec::new();
    }
    let ql = query.trim().to_lowercase();
    let mut scored: Vec<Scored> = candidates
        .iter()
        .filter_map(|c| {
            let slug_n = normalize(&c.slug);
            let name_n = normalize(&c.name);
            let (base, reason) = if slug_n == qn || name_n == qn {
                (100, "exact")
            } else if c.aliases.iter().any(|a| normalize(a) == qn) {
                (85, "alias")
            } else if slug_n.starts_with(&qn) || name_n.starts_with(&qn) {
                (65, "prefix")
            } else if c.name.to_lowercase().contains(&ql)
                || c.aliases.iter().any(|a| a.to_lowercase().contains(&ql))
            {
                (40, "contains")
            } else {
                (0, "")
            };
            if base == 0 {
                return None;
            }
            Some(Scored {
                candidate: c.clone(),
                score: base + c.popularity.min(15) as i32,
                reason,
            })
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(b.candidate.popularity.cmp(&a.candidate.popularity))
            .then(a.candidate.name.len().cmp(&b.candidate.name.len()))
    });
    scored
}

/// The single best match for auto-linking a token, only when confident
/// (exact/alias, or a clear lead over the runner-up). `None` ⇒ show a picker.
pub fn best(query: &str, candidates: &[Candidate]) -> Option<Candidate> {
    let r = rank(query, candidates);
    match (r.first(), r.get(1)) {
        (Some(top), _) if top.score >= 85 => Some(top.candidate.clone()),
        (Some(top), Some(next)) if top.score > next.score + 10 => Some(top.candidate.clone()),
        (Some(top), None) if top.score >= 65 => Some(top.candidate.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(slug: &str, name: &str, aliases: &[&str], pop: i64) -> Candidate {
        Candidate {
            id: format!("node_{slug}"),
            pack: "pest-organisms".into(),
            slug: slug.into(),
            name: name.into(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            node_type: "organism".into(),
            popularity: pop,
        }
    }

    #[test]
    fn normalize_matches_slugify() {
        assert_eq!(normalize("German Cockroach!"), "german-cockroach");
        assert_eq!(normalize("  Norway   Rat "), "norway-rat");
    }

    #[test]
    fn parse_extracts_all_three_sigils() {
        let toks = parse("See @german-cockroach and #rodents plus [[Norway Rat]] today.");
        assert_eq!(toks.len(), 3);
        assert_eq!(toks[0].kind, TagKind::Mention);
        assert_eq!(toks[0].raw, "german-cockroach");
        assert_eq!(toks[1].kind, TagKind::Hashtag);
        assert_eq!(toks[1].raw, "rodents");
        assert_eq!(toks[2].kind, TagKind::Wikilink);
        assert_eq!(toks[2].raw, "Norway Rat");
        // span round-trips
        assert_eq!(&"See @german-cockroach and #rodents plus [[Norway Rat]] today."[toks[0].start..toks[0].end], "@german-cockroach");
    }

    #[test]
    fn parse_ignores_bare_sigils_and_handles_utf8() {
        assert!(parse("email me @ home, costs £5 # not a tag").is_empty());
        // utf8 before a real token must not panic or misalign
        let toks = parse("café ☕ @sülfur done");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].raw, "s"); // stops at non-ascii-alnum (ü)
    }

    #[test]
    fn rank_prefers_exact_then_prefix() {
        let cands = vec![
            cand("german-cockroach", "German Cockroach", &["Blattella germanica"], 9),
            cand("german-cockroach-bait", "German Cockroach Bait", &[], 2),
            cand("american-cockroach", "American Cockroach", &[], 1),
        ];
        let r = rank("german cockroach", &cands);
        assert_eq!(r[0].candidate.slug, "german-cockroach");
        assert_eq!(r[0].reason, "exact");
        // prefix beats unrelated; american-cockroach shouldn't rank for this query
        assert!(r.iter().all(|s| s.candidate.slug != "american-cockroach"));

        // alias resolution
        let r2 = rank("Blattella germanica", &cands);
        assert_eq!(r2[0].candidate.slug, "german-cockroach");
        assert_eq!(r2[0].reason, "alias");
    }

    #[test]
    fn best_is_confident_on_exact_but_not_on_ambiguous_prefix() {
        let cands = vec![
            cand("bait-a", "Bait A", &[], 1),
            cand("bait-b", "Bait B", &[], 1),
        ];
        assert!(best("bait", &cands).is_none(), "ambiguous prefix → picker, not auto-link");
        assert_eq!(best("bait a", &cands).unwrap().slug, "bait-a");
    }

    #[test]
    fn empty_query_ranks_nothing() {
        assert!(rank("   ", &[cand("x", "X", &[], 0)]).is_empty());
    }
}
