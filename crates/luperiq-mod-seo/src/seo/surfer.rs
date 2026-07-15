//! Surfer SEO sheet types, .txt parser, WAL read/write, and directory import.
//!
//! Surfer SEO exports content guidelines as `.txt` files with three sections:
//! CONTENT STRUCTURE, IMPORTANT TERMS TO USE, and FACTS TO INCLUDE.
//! This module parses those files into typed `SurferSheet` values and stores
//! them in the ForgeJournal under the `Surfer:Sheet` aggregate type.

use serde::{Deserialize, Serialize};
use std::path::Path;

use luperiq_forge::{ApexEvent, ForgeJournal};

use super::TOMBSTONE;

// ── Aggregate type constants ──────────────────────────────────────────────────

pub const AGG_SURFER_SHEET: &str = "Surfer:Sheet";

// ── Core types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureRange {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureTargets {
    pub images: StructureRange,
    pub headings: StructureRange,
    pub words: StructureRange,
    pub paragraphs: StructureRange,
    pub characters: StructureRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurferTerm {
    pub term: String,
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurferFactGroup {
    pub group: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurferSheet {
    pub sheet_id: String,
    pub topic: String,
    pub source_file: String,
    pub source_date: String,
    pub industry: String,
    pub structure: StructureTargets,
    pub terms: Vec<SurferTerm>,
    pub facts: Vec<SurferFactGroup>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Surfer SEO `.txt` guideline export.
///
/// `content`  — full UTF-8 text of the file
/// `filename` — bare filename (e.g. `"surfer-guidelines-pest control website-16-03-2026.txt"`)
///
/// Returns a fully-populated `SurferSheet` on success, or an error string
/// describing the first problem encountered.
pub fn parse_surfer_txt(content: &str, filename: &str) -> Result<SurferSheet, String> {
    let topic = extract_topic(filename);
    let source_date = extract_date(filename);
    let industry = derive_industry(&topic);
    let sheet_id = slugify_topic(&topic);

    let mut structure = StructureTargets {
        images: StructureRange {
            min: None,
            max: None,
        },
        headings: StructureRange {
            min: None,
            max: None,
        },
        words: StructureRange {
            min: None,
            max: None,
        },
        paragraphs: StructureRange {
            min: None,
            max: None,
        },
        characters: StructureRange {
            min: None,
            max: None,
        },
    };
    let mut terms: Vec<SurferTerm> = Vec::new();
    let mut facts: Vec<SurferFactGroup> = Vec::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Structure,
        Terms,
        Facts,
    }

    let mut section = Section::None;
    let mut current_fact_group: Option<SurferFactGroup> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // Skip blank lines and description hints.
        if line.is_empty() || line.starts_with('_') {
            continue;
        }

        // Section headers.
        if line == "## CONTENT STRUCTURE" {
            section = Section::Structure;
            continue;
        }
        if line == "## IMPORTANT TERMS TO USE" {
            section = Section::Terms;
            continue;
        }
        if line == "## FACTS TO INCLUDE" {
            // Flush any in-progress fact group.
            if let Some(g) = current_fact_group.take() {
                facts.push(g);
            }
            section = Section::Facts;
            continue;
        }

        match section {
            Section::None => {}

            Section::Structure => {
                // Format: `* Key: min - max`
                if let Some(rest) = line.strip_prefix("* ") {
                    if let Some((key, range)) = rest.split_once(": ") {
                        let (min, max) = parse_range(range);
                        let range_val = StructureRange { min, max };
                        match key.to_lowercase().as_str() {
                            "images" => structure.images = range_val,
                            "headings" => structure.headings = range_val,
                            "words" => structure.words = range_val,
                            "paragraphs" => structure.paragraphs = range_val,
                            "characters" => structure.characters = range_val,
                            _ => {} // unknown key — ignore gracefully
                        }
                    }
                }
            }

            Section::Terms => {
                // Format: `* term text: min - max`
                // The range is always the last `: N - N` segment.
                if let Some(rest) = line.strip_prefix("* ") {
                    if let Some(term) = parse_term_line(rest) {
                        terms.push(term);
                    }
                }
            }

            Section::Facts => {
                if let Some(group_name) = line.strip_prefix("### ") {
                    // Start a new fact group, flushing the previous one.
                    if let Some(g) = current_fact_group.take() {
                        facts.push(g);
                    }
                    current_fact_group = Some(SurferFactGroup {
                        group: group_name.trim().to_string(),
                        items: Vec::new(),
                    });
                } else if let Some(fact_text) = line.strip_prefix("* ") {
                    if let Some(ref mut g) = current_fact_group {
                        g.items.push(fact_text.trim().to_string());
                    }
                }
            }
        }
    }

    // Flush last fact group.
    if let Some(g) = current_fact_group.take() {
        facts.push(g);
    }

    Ok(SurferSheet {
        sheet_id,
        topic,
        source_file: filename.to_string(),
        source_date,
        industry,
        structure,
        terms,
        facts,
    })
}

// ── Parser helpers ────────────────────────────────────────────────────────────

/// Parse a range string like `"36 - 105"` or `"28 - Infinity"`.
///
/// Returns `(min, max)` where `Infinity` maps to `None`.
fn parse_range(s: &str) -> (Option<u64>, Option<u64>) {
    let parts: Vec<&str> = s.splitn(2, " - ").collect();
    if parts.len() != 2 {
        return (None, None);
    }
    let min = parts[0].trim().parse::<u64>().ok();
    let max = if parts[1].trim().eq_ignore_ascii_case("infinity") {
        None
    } else {
        parts[1].trim().parse::<u64>().ok()
    };
    (min, max)
}

/// Parse a term line like `"pest control website: 1 - 1"`.
///
/// The strategy: find the last `: N - N` suffix by scanning for a colon
/// followed by two numbers separated by ` - `.
fn parse_term_line(s: &str) -> Option<SurferTerm> {
    // Find the last colon that is followed by a valid `min - max` pattern.
    let mut last_colon = None;
    for (i, _) in s.match_indices(':') {
        let after = s[i + 1..].trim();
        let parts: Vec<&str> = after.splitn(2, " - ").collect();
        if parts.len() == 2 {
            let ok_min = parts[0].trim().parse::<u32>().is_ok();
            let ok_max = parts[1].trim().parse::<u32>().is_ok()
                || parts[1].trim().eq_ignore_ascii_case("infinity");
            if ok_min && ok_max {
                last_colon = Some(i);
            }
        }
    }

    let colon_pos = last_colon?;
    let term = s[..colon_pos].trim().to_string();
    let range_str = s[colon_pos + 1..].trim();
    let parts: Vec<&str> = range_str.splitn(2, " - ").collect();
    if parts.len() != 2 {
        return None;
    }
    let min = parts[0].trim().parse::<u32>().ok()?;
    let max = if parts[1].trim().eq_ignore_ascii_case("infinity") {
        u32::MAX
    } else {
        parts[1].trim().parse::<u32>().ok()?
    };

    if term.is_empty() {
        return None;
    }

    Some(SurferTerm { term, min, max })
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Derive the industry slug from a topic string.
///
/// The rules intentionally use normalized word/phrase matching instead of raw
/// substring checks. That keeps words like "restaurant" from matching "ant",
/// while still letting narrow future verticals (dental, roofing, locksmith,
/// senior living, etc.) be stored separately from generic Central guidance.
pub fn derive_industry(topic: &str) -> String {
    let t = normalize_for_industry_match(topic);
    if t.is_empty() {
        return String::new();
    }

    if is_location_city_topic(&t) {
        return "location-city".to_string();
    }

    for (slug, phrases) in INDUSTRY_PHRASE_RULES {
        if has_any_phrase(&t, phrases) {
            return (*slug).to_string();
        }
    }

    for (slug, phrases) in EXTRA_INDUSTRY_PHRASE_RULES {
        if has_any_phrase(&t, phrases) {
            return (*slug).to_string();
        }
    }

    if has_any_phrase(&t, CENTRAL_LUPERIQ_PHRASES) {
        return "central-luperiq".to_string();
    }

    String::new()
}

fn normalize_for_industry_match(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_phrase(normalized_topic: &str, phrase: &str) -> bool {
    let normalized_phrase = normalize_for_industry_match(phrase);
    if normalized_phrase.is_empty() {
        return false;
    }

    let topic = format!(" {normalized_topic} ");
    let needle = format!(" {normalized_phrase} ");
    topic.contains(&needle)
}

fn has_any_phrase(normalized_topic: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| has_phrase(normalized_topic, phrase))
}

fn is_location_city_topic(normalized_topic: &str) -> bool {
    if has_any_phrase(normalized_topic, NORTH_TEXAS_CITY_TOPICS) {
        return true;
    }

    normalized_topic.ends_with(" texas") && normalized_topic.split_whitespace().count() <= 4
}

const NORTH_TEXAS_CITY_TOPICS: &[&str] = &[
    "aledo",
    "aledo texas",
    "allen",
    "allen texas",
    "anna",
    "anna texas",
    "arlington",
    "arlington texas",
    "azle",
    "azle texas",
    "bedford",
    "bedford texas",
    "burleson",
    "burleson texas",
    "carrollton",
    "carrollton texas",
    "celina",
    "celina texas",
    "cleburne",
    "cleburne texas",
    "colleyville",
    "colleyville texas",
    "coppell",
    "coppell texas",
    "dallas",
    "dallas texas",
    "decatur",
    "decatur texas",
    "denton",
    "denton texas",
    "ennis",
    "ennis texas",
    "euless",
    "euless texas",
    "fate",
    "fate texas",
    "flower mound",
    "flower mound texas",
    "forney",
    "forney texas",
    "fort worth",
    "fort worth texas",
    "frisco",
    "frisco texas",
    "garland",
    "garland texas",
    "grand prairie",
    "grand prairie texas",
    "grapevine",
    "grapevine texas",
    "haslet",
    "haslet texas",
    "irving",
    "irving texas",
    "keller",
    "keller texas",
    "lewisville",
    "lewisville texas",
    "little elm",
    "little elm texas",
    "mansfield",
    "mansfield texas",
    "mckinney",
    "mckinney texas",
    "melissa",
    "melissa texas",
    "mesquite",
    "mesquite texas",
    "midlothian",
    "midlothian texas",
    "murphy",
    "murphy texas",
    "north richland hills",
    "north richland hills texas",
    "plano",
    "plano texas",
    "princeton",
    "princeton texas",
    "prosper",
    "prosper texas",
    "richardson",
    "richardson texas",
    "roanoke",
    "roanoke texas",
    "rockwall",
    "rockwall texas",
    "royse city",
    "royse city texas",
    "sachse",
    "sachse texas",
    "southlake",
    "southlake texas",
    "the colony",
    "the colony texas",
    "waxahachie",
    "waxahachie texas",
    "weatherford",
    "weatherford texas",
    "wylie",
    "wylie texas",
];

const INDUSTRY_PHRASE_RULES: &[(&str, &[&str])] = &[
    (
        "legal",
        &[
            "adoption lawyer",
            "alimony",
            "annulment",
            "attorney",
            "birth injury",
            "brain injury",
            "breathalyzer",
            "burn injury",
            "car accident",
            "child custody",
            "child support",
            "construction accident",
            "criminal defense",
            "criminal record",
            "divorce",
            "dog bite",
            "domestic violence",
            "dui",
            "dwi",
            "expungement",
            "family law",
            "father s rights",
            "felony",
            "field sobriety",
            "grandparent rights",
            "guardianship",
            "law firm",
            "lawyer",
            "license suspension",
            "marital property",
            "medical malpractice",
            "motorcycle accident",
            "nursing home abuse",
            "paternity",
            "personal injury",
            "prenuptial",
            "property division",
            "slip and fall",
            "spinal cord injury",
            "spousal support",
            "truck accident",
            "uncontested divorce",
            "underage dui",
            "workers compensation",
        ],
    ),
    (
        "senior-living",
        &[
            "alzheimers care",
            "assisted living",
            "continuing care",
            "dementia care",
            "elder care",
            "independent living",
            "memory care",
            "nursing home",
            "residential care",
            "retirement communities",
            "retirement homes",
            "senior assisted",
            "senior care",
            "senior housing",
            "senior living",
            "skilled nursing",
        ],
    ),
    (
        "dental",
        &[
            "braces",
            "clear aligners",
            "cosmetic dentistry",
            "dental",
            "dentist",
            "emergency dentist",
            "family dentist",
            "gum disease",
            "invisalign",
            "orthodontist",
            "pediatric dentist",
            "root canal",
            "smile makeover",
            "teeth whitening",
            "tooth extraction",
            "veneers",
        ],
    ),
    (
        "med-spa",
        &[
            "abdominoplasty",
            "acne treatment",
            "anti aging",
            "blepharoplasty",
            "body contouring",
            "botox",
            "brazilian butt lift",
            "breast augmentation",
            "breast lift",
            "breast reduction",
            "chemical peel",
            "coolsculpting",
            "cosmetic surgeon",
            "cosmetic surgery",
            "dermal fillers",
            "eyelid surgery",
            "facelift",
            "hydrafacial",
            "hyperpigmentation",
            "iv hydration",
            "kybella",
            "laser hair removal",
            "laser skin resurfacing",
            "lip fillers",
            "liposuction",
            "med spa",
            "microneedling",
            "mole removal",
            "mommy makeover",
            "neck lift",
            "oral surgery",
            "plastic surgeon",
            "prp therapy",
            "rhinoplasty",
            "rosacea",
            "scar revision",
            "skin tightening",
            "thread lift",
            "tummy tuck",
        ],
    ),
    (
        "auto-body",
        &[
            "auto body",
            "auto collision",
            "auto painting",
            "body shop",
            "bumper repair",
            "car body",
            "car dent",
            "car paint",
            "clear coat",
            "collision repair",
            "custom paint",
            "dent removal",
            "dented car",
            "fender repair",
            "frame straightening",
            "hail damage",
            "paintless dent",
            "rust repair",
            "scratch repair",
            "windshield replacement",
        ],
    ),
    (
        "auto-sales",
        &[
            "auto loans",
            "bad credit car loans",
            "buy here pay here",
            "buying a car",
            "car deals",
            "car dealership",
            "car financing",
            "car lease",
            "car price",
            "car trade in",
            "certified pre owned",
            "electric cars",
            "hybrid cars",
            "luxury cars",
            "new cars",
            "pre owned vehicles",
            "suv sales",
            "trade in car",
            "truck sales",
            "used cars",
            "used suvs",
            "used trucks",
            "vehicle specials",
        ],
    ),
    ("auto-repair", &["auto repair", "mechanic", "repair shop"]),
    (
        "pest-control",
        &[
            "alpha gal",
            "american cockroach",
            "anaplasmosis",
            "annual pest",
            "ant control",
            "ant exterminator",
            "ant infestation",
            "apartment bed bug",
            "attic rodent",
            "babesiosis",
            "backyard mosquito",
            "bat removal",
            "bed bug",
            "bug exterminator",
            "carpenter ant",
            "chikungunya",
            "cockroach",
            "commercial pest",
            "crawl space rodent",
            "deer tick",
            "dengue",
            "drywood termite",
            "eastern equine",
            "eco friendly pest",
            "ehrlichiosis",
            "emergency pest",
            "exterminator",
            "fire ant",
            "flea",
            "german roach",
            "german roaches",
            "hotel bed bug",
            "house mouse",
            "mosquito",
            "mouse treatment",
            "oriental cockroach",
            "pest control",
            "pest management",
            "pest treatment",
            "rat control",
            "roach",
            "rodent",
            "termite",
            "tick control",
            "tick exterminator",
            "wasp",
        ],
    ),
    (
        "plumbing",
        &[
            "backflow",
            "bathroom plumbing",
            "bathtub drain",
            "burst pipe",
            "clogged drain",
            "clogged toilet",
            "commercial plumbing",
            "dishwasher line",
            "drain cleaning",
            "drain line",
            "drainage solutions",
            "emergency plumber",
            "emergency sewer",
            "emergency water leak",
            "faucet",
            "frozen pipe",
            "garbage disposal",
            "gas leak plumber",
            "gas line plumber",
            "gas line repair",
            "hard water",
            "hot water",
            "hydro jetting",
            "ice maker line",
            "kitchen plumbing",
            "leak detection",
            "local plumbing",
            "low water pressure",
            "outdoor faucet",
            "pipe",
            "plumber",
            "plumbing",
            "sewer",
            "sump pump",
            "toilet",
            "water heater",
            "water leak",
            "water pressure",
        ],
    ),
    (
        "hvac",
        &[
            "ac compressor",
            "ac installation",
            "ac maintenance",
            "ac not cooling",
            "ac repair",
            "ac replacement",
            "ac tune up",
            "air balancing",
            "air conditioner",
            "air conditioning",
            "air duct",
            "apartment hvac",
            "boiler repair",
            "central air",
            "commercial hvac",
            "condenser repair",
            "ductless",
            "emergency ac",
            "emergency hvac",
            "emergency no heat",
            "evaporator coil",
            "furnace",
            "heat pump",
            "heating repair",
            "home airflow",
            "hvac",
            "lower cooling bills",
        ],
    ),
    (
        "electrical",
        &[
            "afci",
            "aluminum wiring",
            "appliance circuit",
            "attic fan",
            "backup generator",
            "bathroom exhaust fan",
            "breaker",
            "burning smell from outlet",
            "car charger electrician",
            "carbon monoxide detector",
            "ceiling fan",
            "circuit",
            "code correction electrician",
            "commercial electrician",
            "commercial lighting",
            "data cabling",
            "dedicated circuit",
            "dimmer switch",
            "electrical",
            "electrician",
            "ev charger",
            "flickering lights",
            "gfci",
            "home rewiring",
            "hot tub electrical",
            "knob and tube",
            "landscape lighting",
            "light fixture",
            "low voltage",
            "meter base",
            "outdoor lighting",
            "outlet",
            "panel replacement",
            "panel upgrade",
            "parking lot lighting",
            "pool electrical",
            "property management electrical",
            "rewire",
            "wiring",
        ],
    ),
    (
        "roofing",
        &[
            "flat roof",
            "metal roofing",
            "new roof",
            "roof",
            "roofing",
            "shingle",
            "storm damage",
            "tile roof",
        ],
    ),
    (
        "restoration",
        &[
            "basement water damage",
            "ceiling water damage",
            "emergency water removal",
            "fire and water restoration",
            "flood cleanup",
            "flooded basement",
            "hardwood floor water",
            "mold remediation",
            "mold removal",
            "sewage cleanup",
            "water damage",
            "water extraction",
            "water mitigation",
        ],
    ),
    (
        "locksmith",
        &[
            "automotive locksmith",
            "broken key",
            "car key",
            "car lockout",
            "commercial locksmith",
            "deadbolt",
            "emergency locksmith",
            "high security locks",
            "house lockout",
            "key duplication",
            "key fob",
            "keyring",
            "lock installation",
            "lock rekey",
            "locked out",
            "locksmith",
            "lockout service",
            "mailbox lock",
            "safe opening",
            "smart lock",
            "transponder key",
        ],
    ),
    (
        "moving",
        &[
            "affordable movers",
            "apartment movers",
            "cheap moving",
            "commercial movers",
            "cross country movers",
            "full service moving",
            "furniture movers",
            "hire movers",
            "international movers",
            "interstate movers",
            "local movers",
            "long distance movers",
            "moving and storage",
            "moving companies",
            "moving company",
            "moving quotes",
            "moving services",
            "office movers",
            "packing services",
            "piano movers",
            "professional movers",
        ],
    ),
    (
        "real-estate",
        &[
            "buy a home",
            "buyer s agent",
            "condos for sale",
            "first time home buyer",
            "home evaluation",
            "home search",
            "home values",
            "houses for sale",
            "investment property",
            "land for sale",
            "luxury homes",
            "mortgage broker",
            "new homes",
            "property listings",
            "real estate",
            "realtor",
            "sell my home",
            "townhomes for sale",
        ],
    ),
    (
        "landscaping",
        &[
            "backyard landscaping",
            "commercial landscaping",
            "fall leaf cleanup",
            "fertilization service",
            "front yard landscaping",
            "irrigation",
            "landscape design",
            "landscape lighting",
            "landscaper",
            "landscaping",
            "lawn care",
            "mulch installation",
            "outdoor lighting landscaping",
            "tree trimming",
            "yard drainage",
        ],
    ),
    (
        "restaurant-food",
        &[
            "brewery website",
            "cafe website",
            "catering",
            "distillery website",
            "food business",
            "food truck",
            "personal chef",
            "restaurant",
        ],
    ),
    ("bakery", &["bakery"]),
    ("coffee-shop", &["coffee shop"]),
    (
        "salon",
        &[
            "barbershop",
            "crawl space cleanup",
            "local newspaper",
            "massage therapist",
            "nail salon",
            "salon",
            "spa website",
        ],
    ),
    (
        "education-coaching",
        &[
            "dance studio",
            "e learning",
            "education website",
            "learning platform",
            "martial arts",
            "music school",
            "music students",
            "online course",
            "online learning",
            "quiz and assignment",
            "sports coaching",
            "student login",
            "tutoring",
        ],
    ),
    (
        "fitness",
        &["fitness", "gym", "personal training", "yoga studio"],
    ),
    (
        "community-orgs",
        &[
            "church website",
            "community theater",
            "community website",
            "faith based",
            "gaming community",
            "hoa",
            "membership site",
            "membership website",
            "neighborhood",
            "nonprofit",
        ],
    ),
    (
        "creator-blog",
        &[
            "blog and website",
            "blogger",
            "content creators",
            "creator website",
            "gardening blog",
            "personal brand",
            "podcast",
            "portfolio",
        ],
    ),
    ("music-band", &["dj or band", "musician"]),
    (
        "creative-commerce",
        &[
            "art studio",
            "artwork",
            "bookstore",
            "candle",
            "craft",
            "diaper bag",
            "e commerce",
            "etsy",
            "florist",
            "gift shop",
            "handmade",
            "hobby to business",
            "insulated wine bags",
            "jewelry",
            "laptop gifts",
            "laptop sleeve",
            "leather",
            "legal pad",
            "maker marketplace",
            "makeup bags",
            "monogram",
            "patches",
            "photography",
            "pottery",
            "purse",
            "quilting",
            "retail",
            "scrapbooking",
            "small shop",
            "soap",
            "storefront",
            "tattoo shop",
            "woocommerce",
            "woodworking",
        ],
    ),
    (
        "construction-remodeling",
        &[
            "construction company",
            "general contractor",
            "remodeling contractor",
        ],
    ),
    ("appliance-repair", &["appliance repair"]),
    ("childcare", &["childcare"]),
    ("cleaning", &["cleaning company"]),
    ("fence-company", &["fence company"]),
    ("garage-door", &["garage door"]),
    ("handyman", &["handyman"]),
    ("painting", &["painting company"]),
    ("pool-service", &["pool service"]),
    ("pressure-washing", &["pressure washing"]),
    ("security", &["security company"]),
    (
        "travel-hospitality",
        &[
            "airbnb",
            "bed and breakfast",
            "hotel",
            "resort",
            "tour guide",
            "travel agency",
        ],
    ),
    ("window-cleaning", &["window cleaning"]),
];

const EXTRA_INDUSTRY_PHRASE_RULES: &[(&str, &[&str])] = &[
    (
        "pest-control",
        &[
            "cockroaches",
            "get rid of ants",
            "get rid of bed bugs",
            "get rid of fleas",
            "get rid of roaches",
            "get rid of ticks",
            "home pest inspection",
            "hornet",
            "house bug",
            "insect control",
            "lyme disease",
            "malaria",
            "mouse control",
            "odorous house ant",
            "palmetto bug",
            "pest inspection",
            "pest prevention",
            "pest removal",
            "pharaoh ant",
            "powassan virus",
            "raccoon removal",
            "rocky mountain spotted fever",
            "signs of bed bugs",
            "signs of lyme disease",
            "signs of termites",
            "silverfish",
            "spider control",
            "squirrel removal",
            "st louis encephalitis",
            "sugar ant",
            "swarming termites",
            "tick bite",
            "tick treatment",
            "tick yard",
            "remove a tick",
            "west nile",
            "wildlife removal",
        ],
    ),
    (
        "plumbing",
        &[
            "repiping",
            "shower valve",
            "sink draining",
            "slab leak",
            "unclog a drain",
            "utility sink",
            "water line",
            "water softener",
        ],
    ),
    (
        "hvac",
        &[
            "indoor air quality",
            "mini split",
            "refrigerant leak",
            "rooftop unit",
            "smart thermostat",
            "thermostat",
            "uneven rooms",
            "ventilation",
            "whole home air purifier",
            "whole home humidifier",
            "what to do when your ac stops working",
            "why is my ac",
        ],
    ),
    (
        "electrical",
        &[
            "generator installation",
            "generator transfer switch",
            "recessed lighting",
            "security lighting",
            "service mast",
            "smoke detector",
            "standby generator",
            "subpanel",
            "surge protection",
            "switch replacement",
            "transfer switch",
            "under cabinet lighting",
            "whole home generator",
            "whole home surge",
            "whole house rewiring",
            "whole house surge",
            "power goes out",
        ],
    ),
    (
        "restoration",
        &[
            "flood damage restoration",
            "how long does it take to dry out a flooded home",
            "what to do if your basement floods",
        ],
    ),
    (
        "real-estate",
        &[
            "comparative market analysis",
            "buying and renting a home",
            "credit score do you need to buy a house",
        ],
    ),
    (
        "landscaping",
        &[
            "lawn aeration",
            "lawn mowing",
            "overseeding",
            "patio installation",
            "retaining wall",
            "seasonal yard",
            "shrub trimming",
            "sod installation",
            "spring yard",
            "sprinkler",
            "standing water in yard",
            "weed control",
        ],
    ),
    (
        "locksmith",
        &[
            "ignition interlock",
            "ignition repair",
            "lock your keys in your car",
            "rekeying and replacing locks",
            "wallet attached to keys",
        ],
    ),
    ("legal", &["legal separation"]),
    ("med-spa", &["hair transplant", "plastic surgery"]),
    (
        "auto-sales",
        &[
            "should i buy or lease a car",
            "what to look for when buying a used car",
        ],
    ),
    ("auto-body", &["file an insurance claim for car damage"]),
    (
        "moving",
        &["residential movers", "what to look for when hiring movers"],
    ),
    (
        "artisan-market",
        &["artisan market", "farmers market", "food co op"],
    ),
    (
        "restaurant-food",
        &[
            "online menu",
            "online ordering",
            "restaurants and food businesses",
            "handle online reviews for restaurants",
        ],
    ),
    (
        "salon",
        &[
            "instagram marketing for salons",
            "makeup artist",
            "personal stylist",
        ],
    ),
    (
        "education-coaching",
        &[
            "coaches",
            "coaching programs",
            "educational content",
            "learner portal",
            "life coach",
            "private lessons",
            "teaching online",
            "tutors",
            "workshops and classes",
        ],
    ),
    ("fitness", &["personal trainer"]),
    (
        "community-orgs",
        &[
            "animal rescue",
            "book club",
            "build a community around your hobby",
            "political campaign",
        ],
    ),
    (
        "creator-blog",
        &[
            "author or writer",
            "freelancers",
            "grow a following as a creator",
            "hobby business",
            "my hobby business",
            "side hustle",
            "solo entrepreneur",
            "start a blog about your hobby",
            "vintage collector",
        ],
    ),
    (
        "sports-team",
        &["fantasy sports league", "youth sports league"],
    ),
    (
        "pet-services",
        &["dog trainer", "horse boarding", "pet groomer"],
    ),
    (
        "professional-services",
        &[
            "bookkeeper",
            "business consultant",
            "financial planner",
            "recruiting firm",
            "staffing agency",
            "tax preparer",
        ],
    ),
    ("events", &["event planner", "wedding photographer"]),
    ("home-inspection", &["home inspector"]),
    ("music-band", &["music producer"]),
    (
        "creative-commerce",
        &[
            "audience for your hobby",
            "cheap personalized cosmetic bags",
            "crafts and diy",
            "make money from a hobby",
            "sell candles",
            "sell digital products",
            "sell homemade food",
            "sell plants",
            "turn a hobby into a business",
            "wallet with ring",
            "website for artist",
            "website for photographer",
            "website for sports memorabilia",
            "website with cart and checkout",
        ],
    ),
    (
        "medical-office",
        &[
            "acupuncturist",
            "home health care",
            "medical office",
            "nutritionist",
            "occupational therapist",
            "physical therapist",
            "speech therapist",
            "therapist",
        ],
    ),
];

const CENTRAL_LUPERIQ_PHRASES: &[&str] = &[
    "accounting firm website builder",
    "agency cms platform",
    "ai analytics",
    "ai business solutions",
    "ai local landing page generator",
    "ai powered marketing",
    "ai powered website",
    "ai site setup",
    "ai website generator",
    "all in one wordpress",
    "appointment reminder software",
    "booking system",
    "business management",
    "cms alternative",
    "cms with",
    "contractor website with",
    "customer portal for service business",
    "customer self service",
    "dispatch software",
    "email marketing",
    "enterprise wordpress",
    "facebook",
    "fast cms",
    "field service",
    "followers on x",
    "google business profile",
    "google maps",
    "headless cms",
    "how to build a service route",
    "how to collect payment",
    "how to deal with difficult customers",
    "how to do estimates",
    "how to get 5 star reviews",
    "how to get commercial contracts",
    "how to get more customers",
    "how to get more google reviews",
    "how to get more yelp reviews",
    "how to get repeat customers",
    "how to get sponsors",
    "how to handle no shows",
    "how to grow your linkedin",
    "how to hire good technicians",
    "how to manage service technicians",
    "how to optimize google",
    "how to optimize your yelp",
    "how to rank higher",
    "how to reduce truck rolls",
    "how to respond to yelp",
    "how to run a service business",
    "how to scale a service business",
    "how to start a youtube channel",
    "how to track technicians",
    "how to upsell customers",
    "how to use instagram reels",
    "how to use linkedin",
    "how to use pinterest",
    "how to use tiktok",
    "how to use youtube",
    "how to write a business newsletter",
    "invoicing",
    "linkedin",
    "loyalty program",
    "local business",
    "marketing analytics",
    "marketing dashboard",
    "marketing software",
    "multi site website",
    "online booking for contractors",
    "pinterest marketing",
    "service business",
    "service area page software",
    "service company",
    "service management",
    "small business",
    "tiktok",
    "technician management",
    "website builder",
    "website that writes its own content",
    "website to sell my services",
    "website with online scheduling",
    "wordpress",
    "yelp reviews",
];

/// Convert a topic string into a URL-safe identifier slug.
///
/// Example: `"Pest Control Website"` → `"pest-control-website"`
pub fn slugify_topic(topic: &str) -> String {
    topic
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Strip extension and optional duplicate suffix from a Surfer filename.
///
/// `"surfer-guidelines-foo-16-03-2026 (1).txt"` → `"surfer-guidelines-foo-16-03-2026"`
fn strip_ext_and_dup(filename: &str) -> &str {
    let s = filename.strip_suffix(".txt").unwrap_or(filename);
    // Trim a trailing ` (N)` where N is one or more digits — but do NOT strip
    // digits that are part of the year.  We detect this as a suffix matching
    // the literal pattern " (N…)".
    if let Some(pos) = s.rfind(" (") {
        let tail = &s[pos..]; // e.g. " (1)"
        if tail.starts_with(" (") && tail.ends_with(')') {
            let inner = &tail[2..tail.len() - 1];
            if inner.chars().all(|c| c.is_ascii_digit()) {
                return &s[..pos];
            }
        }
    }
    s
}

/// Extract the human-readable topic from a Surfer export filename.
///
/// Input:  `"surfer-guidelines-pest control website-16-03-2026.txt"`
/// Output: `"pest control website"`
pub fn extract_topic(filename: &str) -> String {
    let name = strip_ext_and_dup(filename);

    // Remove `surfer-guidelines-` prefix.
    let after_prefix = name.strip_prefix("surfer-guidelines-").unwrap_or(name);

    // The date at the end is `-DD-MM-YYYY`. Split by '-', then skip the last
    // three parts that are all numeric (day, month, year).
    let parts: Vec<&str> = after_prefix.split('-').collect();
    let mut skip = 0usize;
    for part in parts.iter().rev() {
        if skip >= 3 {
            break;
        }
        if part.chars().all(|c| c.is_ascii_digit()) && !part.is_empty() {
            skip += 1;
        } else {
            break;
        }
    }

    // Re-join with spaces (the original has spaces converted to `-` by the
    // filename, but topic words separated by spaces look right).
    parts[..parts.len() - skip].join(" ")
}

/// Extract the ISO date from a Surfer export filename.
///
/// Input:  `"surfer-guidelines-pest control website-16-03-2026.txt"`
/// Output: `"2026-03-16"`
pub fn extract_date(filename: &str) -> String {
    let name = strip_ext_and_dup(filename);

    // Last three dash-separated numeric segments are DD-MM-YYYY.
    let parts: Vec<&str> = name.split('-').collect();
    let n = parts.len();
    if n >= 3 {
        let dd = parts[n - 3];
        let mm = parts[n - 2];
        let yyyy = parts[n - 1];
        if !dd.is_empty()
            && !mm.is_empty()
            && !yyyy.is_empty()
            && dd.chars().all(|c| c.is_ascii_digit())
            && mm.chars().all(|c| c.is_ascii_digit())
            && yyyy.chars().all(|c| c.is_ascii_digit())
        {
            return format!("{yyyy}-{mm:0>2}-{dd:0>2}");
        }
    }
    String::new()
}

// ── WAL operations ────────────────────────────────────────────────────────────

/// Persist a `SurferSheet` to the journal.
///
/// Uses `sheet.sheet_id` as the aggregate ID so subsequent saves overwrite the
/// same aggregate (latest-event semantics).
pub fn save_sheet(journal: &mut ForgeJournal, sheet: &SurferSheet) -> Result<(), String> {
    let payload = serde_json::to_vec(sheet).map_err(|e| format!("Serialize SurferSheet: {e}"))?;
    let event = ApexEvent::new(AGG_SURFER_SHEET, &sheet.sheet_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append SurferSheet: {e}"))?;
    Ok(())
}

/// Write a tombstone event for the given `sheet_id`, logically deleting it.
pub fn delete_sheet(journal: &mut ForgeJournal, sheet_id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_SURFER_SHEET, sheet_id, TOMBSTONE.to_vec());
    journal
        .append(event)
        .map_err(|e| format!("Journal append tombstone: {e}"))?;
    Ok(())
}

/// Load all non-deleted `SurferSheet` values from the journal.
pub fn load_all_sheets(journal: &ForgeJournal) -> Vec<SurferSheet> {
    journal
        .latest_by_aggregate_type(AGG_SURFER_SHEET)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .collect()
}

/// Load a single `SurferSheet` by its `sheet_id`. Returns `None` if not found
/// or if the latest event is a tombstone.
pub fn load_sheet(journal: &ForgeJournal, sheet_id: &str) -> Option<SurferSheet> {
    let event = journal.get_latest(AGG_SURFER_SHEET, sheet_id)?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

// ── Directory import ──────────────────────────────────────────────────────────

/// Read every `.txt` file in `dir_path`, parse it as a Surfer export, and
/// persist each resulting sheet to the journal.
///
/// Returns a summary `(imported, errors)`:
/// - `imported` — number of successfully imported sheets
/// - `errors`   — list of `(filename, error_message)` for any failures
pub fn import_directory(
    journal: &mut ForgeJournal,
    dir_path: &Path,
) -> (usize, Vec<(String, String)>) {
    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(err) => {
            return (0, vec![("(directory)".to_string(), err.to_string())]);
        }
    };

    let mut errors: Vec<(String, String)> = Vec::new();
    let mut paths: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    let mut parsed_by_id = std::collections::BTreeMap::<String, SurferSheet>::new();

    for path in paths {
        let fname = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !fname.ends_with(".txt") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                errors.push((fname, err.to_string()));
                continue;
            }
        };

        match parse_surfer_txt(&content, &fname) {
            Ok(sheet) => match parsed_by_id.get(&sheet.sheet_id) {
                Some(existing) if existing.source_date > sheet.source_date => {}
                Some(existing)
                    if existing.source_date == sheet.source_date
                        && existing.source_file.as_str() <= sheet.source_file.as_str() => {}
                _ => {
                    parsed_by_id.insert(sheet.sheet_id.clone(), sheet);
                }
            },
            Err(e) => errors.push((fname, e)),
        }
    }

    let mut imported = 0usize;
    for sheet in parsed_by_id.values() {
        match save_sheet(journal, sheet) {
            Ok(()) => imported += 1,
            Err(e) => errors.push((sheet.source_file.clone(), e)),
        }
    }

    (imported, errors)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_date ─────────────────────────────────────────────────────────

    #[test]
    fn test_extract_date_standard() {
        assert_eq!(
            extract_date("surfer-guidelines-pest control website-16-03-2026.txt"),
            "2026-03-16"
        );
    }

    #[test]
    fn test_extract_date_hvac() {
        assert_eq!(
            extract_date("surfer-guidelines-hvac seo-30-03-2026.txt"),
            "2026-03-30"
        );
    }

    #[test]
    fn test_extract_date_duplicate_suffix() {
        // Files like "...-16-03-2026 (1).txt" should still parse correctly.
        assert_eq!(
            extract_date("surfer-guidelines-fort worth-09-03-2026 (1).txt"),
            "2026-03-09"
        );
    }

    // ── extract_topic ─────────────────────────────────────────────────────────

    #[test]
    fn test_extract_topic_standard() {
        assert_eq!(
            extract_topic("surfer-guidelines-pest control website-16-03-2026.txt"),
            "pest control website"
        );
    }

    #[test]
    fn test_extract_topic_hvac() {
        assert_eq!(
            extract_topic("surfer-guidelines-hvac seo-30-03-2026.txt"),
            "hvac seo"
        );
    }

    #[test]
    fn test_extract_topic_fort_worth() {
        assert_eq!(
            extract_topic("surfer-guidelines-fort worth-09-03-2026.txt"),
            "fort worth"
        );
    }

    #[test]
    fn test_extract_topic_how_to() {
        assert_eq!(
            extract_topic("surfer-guidelines-how to create pest control invoices-16-03-2026.txt"),
            "how to create pest control invoices"
        );
    }

    // ── derive_industry ───────────────────────────────────────────────────────

    #[test]
    fn test_derive_industry_pest_control() {
        assert_eq!(derive_industry("pest control website"), "pest-control");
    }

    #[test]
    fn test_derive_industry_hvac() {
        assert_eq!(derive_industry("hvac seo"), "hvac");
    }

    #[test]
    fn test_derive_industry_electrical() {
        assert_eq!(derive_industry("electrical invoicing"), "electrical");
    }

    #[test]
    fn test_derive_industry_plumbing() {
        assert_eq!(derive_industry("plumbing website design"), "plumbing");
    }

    #[test]
    fn test_derive_industry_fort_worth_empty() {
        assert_eq!(derive_industry("fort worth"), "location-city");
    }

    #[test]
    fn test_derive_industry_german_roaches() {
        assert_eq!(derive_industry("german roaches"), "pest-control");
    }

    #[test]
    fn test_derive_industry_coffee_shop() {
        assert_eq!(derive_industry("coffee shop website"), "coffee-shop");
    }

    #[test]
    fn test_derive_industry_how_to_pest() {
        assert_eq!(
            derive_industry("how to create pest control invoices"),
            "pest-control"
        );
    }

    #[test]
    fn test_derive_industry_salon() {
        assert_eq!(derive_industry("salon website"), "salon");
    }

    #[test]
    fn test_derive_industry_does_not_match_ant_inside_restaurant() {
        assert_eq!(derive_industry("restaurant website"), "restaurant-food");
        assert_eq!(
            derive_industry("antique shop website builder"),
            "central-luperiq"
        );
        assert_eq!(derive_industry("anti aging treatment"), "med-spa");
    }

    #[test]
    fn test_derive_industry_future_verticals() {
        assert_eq!(derive_industry("roof replacement"), "roofing");
        assert_eq!(derive_industry("dui lawyer"), "legal");
        assert_eq!(
            derive_industry("assisted living facilities"),
            "senior-living"
        );
        assert_eq!(derive_industry("auto body repair"), "auto-body");
        assert_eq!(derive_industry("botox injections"), "med-spa");
        assert_eq!(derive_industry("root canal"), "dental");
    }

    #[test]
    fn test_derive_industry_new_pest_and_city_topics() {
        assert_eq!(derive_industry("rat control near me"), "pest-control");
        assert_eq!(derive_industry("tick exterminator near me"), "pest-control");
        assert_eq!(derive_industry("ant control near me"), "pest-control");
        assert_eq!(derive_industry("fort worth texas"), "location-city");
    }

    #[test]
    fn test_derive_industry_generic_luperiq_acquisition_topics() {
        assert_eq!(
            derive_industry("website builder with seo built in"),
            "central-luperiq"
        );
        assert_eq!(derive_industry("ai website generator"), "central-luperiq");
    }

    // ── slugify_topic ─────────────────────────────────────────────────────────

    #[test]
    fn test_slugify_topic() {
        assert_eq!(
            slugify_topic("pest control website"),
            "pest-control-website"
        );
        assert_eq!(slugify_topic("hvac seo"), "hvac-seo");
        assert_eq!(slugify_topic("fort worth"), "fort-worth");
    }

    // ── parse_range ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_range_normal() {
        assert_eq!(parse_range("36 - 105"), (Some(36), Some(105)));
    }

    #[test]
    fn test_parse_range_infinity() {
        assert_eq!(parse_range("28 - Infinity"), (Some(28), None));
    }

    // ── parse_term_line ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_term_simple() {
        let t = parse_term_line("pest control: 60 - 164").unwrap();
        assert_eq!(t.term, "pest control");
        assert_eq!(t.min, 60);
        assert_eq!(t.max, 164);
    }

    #[test]
    fn test_parse_term_with_colon_in_name() {
        // Edge case: term text itself has a colon (e.g. "NAP: Name, Address, Phone: 1 - 2")
        // Parser should use the last valid `: N - N` segment.
        let t = parse_term_line("google business profile: 5 - 9").unwrap();
        assert_eq!(t.term, "google business profile");
        assert_eq!(t.min, 5);
        assert_eq!(t.max, 9);
    }

    #[test]
    fn test_parse_term_one_one() {
        let t = parse_term_line("pest control website: 1 - 1").unwrap();
        assert_eq!(t.term, "pest control website");
        assert_eq!(t.min, 1);
        assert_eq!(t.max, 1);
    }

    // ── parse_surfer_txt (integration) ───────────────────────────────────────

    const MINI_TXT: &str = r#"## CONTENT STRUCTURE
* Images: 36 - 105
* Headings: 18 - 30
* Characters: 24018 - 105544
* Paragraphs: 28 - Infinity
* Words: 4006 - 4607

## IMPORTANT TERMS TO USE
_Make sure to include those as many times as stated._
* pest control website: 1 - 1
* pest control: 60 - 164
* pest management: 13 - 29

## FACTS TO INCLUDE
_Facts are grouped by topics._
### DIY Pest Control Solutions
* DoMyOwn.com offers professional-grade products for DIY pest control.
* Effective pest control websites include educational resources and professional service options.
### Customized Pest Control Services
* Customized pest control programs are designed based on location, climate, landscaping, and common pests in the area.
"#;

    #[test]
    fn test_parse_mini_structure() {
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();

        assert_eq!(sheet.topic, "pest control website");
        assert_eq!(sheet.source_date, "2026-03-16");
        assert_eq!(sheet.industry, "pest-control");
        assert_eq!(sheet.sheet_id, "pest-control-website");

        // Structure
        assert_eq!(
            sheet.structure.images,
            StructureRange {
                min: Some(36),
                max: Some(105)
            }
        );
        assert_eq!(
            sheet.structure.headings,
            StructureRange {
                min: Some(18),
                max: Some(30)
            }
        );
        assert_eq!(
            sheet.structure.paragraphs,
            StructureRange {
                min: Some(28),
                max: None
            }
        );
        assert_eq!(
            sheet.structure.words,
            StructureRange {
                min: Some(4006),
                max: Some(4607)
            }
        );
        assert_eq!(
            sheet.structure.characters,
            StructureRange {
                min: Some(24018),
                max: Some(105544)
            }
        );
    }

    #[test]
    fn test_parse_mini_terms() {
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();

        assert_eq!(sheet.terms.len(), 3);
        assert_eq!(sheet.terms[0].term, "pest control website");
        assert_eq!(sheet.terms[0].min, 1);
        assert_eq!(sheet.terms[0].max, 1);
        assert_eq!(sheet.terms[1].term, "pest control");
        assert_eq!(sheet.terms[1].min, 60);
        assert_eq!(sheet.terms[1].max, 164);
    }

    #[test]
    fn test_parse_mini_facts() {
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();

        assert_eq!(sheet.facts.len(), 2);
        assert_eq!(sheet.facts[0].group, "DIY Pest Control Solutions");
        assert_eq!(sheet.facts[0].items.len(), 2);
        assert_eq!(sheet.facts[1].group, "Customized Pest Control Services");
        assert_eq!(sheet.facts[1].items.len(), 1);
    }

    // ── WAL round-trip ────────────────────────────────────────────────────────

    fn make_test_journal() -> (ForgeJournal, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let journal = ForgeJournal::open(wal, snap, luperiq_forge::DurabilityMode::Sync).unwrap();
        (journal, dir)
    }

    #[test]
    fn test_wal_save_and_load() {
        let (mut journal, _dir) = make_test_journal();
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();

        save_sheet(&mut journal, &sheet).unwrap();
        let loaded = load_sheet(&journal, "pest-control-website").unwrap();
        assert_eq!(loaded, sheet);
    }

    #[test]
    fn test_wal_load_all() {
        let (mut journal, _dir) = make_test_journal();
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();
        save_sheet(&mut journal, &sheet).unwrap();

        let all = load_all_sheets(&journal);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].sheet_id, "pest-control-website");
    }

    #[test]
    fn test_wal_delete_tombstone() {
        let (mut journal, _dir) = make_test_journal();
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();
        save_sheet(&mut journal, &sheet).unwrap();
        delete_sheet(&mut journal, "pest-control-website").unwrap();

        assert!(load_sheet(&journal, "pest-control-website").is_none());
        assert!(load_all_sheets(&journal).is_empty());
    }

    #[test]
    fn test_wal_overwrite_same_id() {
        let (mut journal, _dir) = make_test_journal();
        let mut sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();
        save_sheet(&mut journal, &sheet).unwrap();

        sheet.industry = "updated-industry".to_string();
        save_sheet(&mut journal, &sheet).unwrap();

        let loaded = load_sheet(&journal, "pest-control-website").unwrap();
        assert_eq!(loaded.industry, "updated-industry");

        // load_all_sheets should still return only one entry (latest-event semantics).
        assert_eq!(load_all_sheets(&journal).len(), 1);
    }

    // ── Serialization round-trip ──────────────────────────────────────────────

    #[test]
    fn test_serialization_roundtrip() {
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();
        let json = serde_json::to_string(&sheet).unwrap();
        let deserialized: SurferSheet = serde_json::from_str(&json).unwrap();
        assert_eq!(sheet, deserialized);
    }
}
