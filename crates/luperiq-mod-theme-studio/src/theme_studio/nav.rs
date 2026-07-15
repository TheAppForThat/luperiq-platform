//! Navigation menu storage -- ForgeJournal CRUD for nav menu trees.
//!
//! Each menu is stored as a single aggregate keyed by its `location` slug
//! (e.g. "header", "footer", "mobile").  Deletion uses the Theme Studio
//! tombstone sentinel from [`super::config::TOMBSTONE`].

use super::config::{NavMenu, NavMenuItem, AGG_NAV_MENU, TOMBSTONE};
use luperiq_forge::{ApexEvent, ForgeJournal, PlatformError};

// ── CRUD ────────────────────────────────────────────────────────────────

/// Load a navigation menu by location.
pub fn get_menu(journal: &ForgeJournal, location: &str) -> Result<Option<NavMenu>, PlatformError> {
    match journal.get_latest(AGG_NAV_MENU, location) {
        Some(event) if event.payload == TOMBSTONE => Ok(None),
        Some(event) => {
            let menu: NavMenu = serde_json::from_slice(&event.payload)
                .map_err(|e| PlatformError::Serialization(e.to_string()))?;
            Ok(Some(menu))
        }
        None => Ok(None),
    }
}

/// Save a navigation menu (full replacement).
pub fn save_menu(journal: &mut ForgeJournal, menu: &NavMenu) -> Result<(), PlatformError> {
    let bytes =
        serde_json::to_vec(menu).map_err(|e| PlatformError::Serialization(e.to_string()))?;
    journal.append(ApexEvent::new(AGG_NAV_MENU, &menu.location, bytes))?;
    Ok(())
}

/// Delete a navigation menu (tombstone).
pub fn delete_menu(journal: &mut ForgeJournal, location: &str) -> Result<(), PlatformError> {
    journal.append(ApexEvent::new(AGG_NAV_MENU, location, TOMBSTONE.to_vec()))?;
    Ok(())
}

/// List all non-deleted navigation menus.
pub fn list_menus(journal: &ForgeJournal) -> Result<Vec<NavMenu>, PlatformError> {
    Ok(journal
        .latest_by_aggregate_type(AGG_NAV_MENU)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<NavMenu>(&e.payload).ok())
        .collect())
}

/// List all saved navigation structures (locations starting with "structure:").
/// This is separate from `list_menus()` which returns operational menus
/// (primary, footer, etc.). Structure names are the part after "structure:".
pub fn list_structures(journal: &ForgeJournal) -> Result<Vec<NavMenu>, PlatformError> {
    Ok(journal
        .latest_by_aggregate_type(AGG_NAV_MENU)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<NavMenu>(&e.payload).ok())
        .filter(|m| m.location.starts_with("structure:"))
        .collect())
}

/// Add a single item to an existing menu.  Creates the menu if it does not exist.
pub fn add_item(
    journal: &mut ForgeJournal,
    location: &str,
    item: NavMenuItem,
) -> Result<(), PlatformError> {
    let mut menu = get_menu(journal, location)?.unwrap_or(NavMenu {
        location: location.to_string(),
        items: vec![],
    });
    menu.items.push(item);
    save_menu(journal, &menu)
}

/// Remove an item by `item_id` from a menu.  Returns `false` if the menu
/// does not exist or the item was not found.
pub fn remove_item(
    journal: &mut ForgeJournal,
    location: &str,
    item_id: &str,
) -> Result<bool, PlatformError> {
    let mut menu = match get_menu(journal, location)? {
        Some(m) => m,
        None => return Ok(false),
    };
    let before = menu.items.len();
    menu.items.retain(|i| i.item_id != item_id);
    if menu.items.len() == before {
        return Ok(false);
    }
    save_menu(journal, &menu)?;
    Ok(true)
}

// ── Tree builder ────────────────────────────────────────────────────────

/// A node in a hierarchical nav tree (item + recursive children).
#[derive(Debug, Clone)]
pub struct NavTreeNode {
    pub item: NavMenuItem,
    pub children: Vec<NavTreeNode>,
}

/// Build a parent-child tree from a flat list of nav items.
///
/// Returns top-level items (those with `parent_id == None`) with their
/// children recursively nested.  Items at each level are sorted by
/// `position`.  This is the primary entry point used by mega-nav rendering.
pub fn build_tree(items: &[NavMenuItem]) -> Vec<NavTreeNode> {
    let mut top: Vec<&NavMenuItem> = items.iter().filter(|i| i.parent_id.is_none()).collect();
    top.sort_by_key(|i| i.position);
    top.into_iter()
        .map(|item| NavTreeNode {
            item: item.clone(),
            children: collect_children(items, &item.item_id),
        })
        .collect()
}

fn collect_children(items: &[NavMenuItem], parent_id: &str) -> Vec<NavTreeNode> {
    let mut children: Vec<&NavMenuItem> = items
        .iter()
        .filter(|i| i.parent_id.as_deref() == Some(parent_id))
        .collect();
    children.sort_by_key(|i| i.position);
    children
        .into_iter()
        .map(|item| NavTreeNode {
            item: item.clone(),
            children: collect_children(items, &item.item_id),
        })
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::config::{NavMenu, NavMenuItem};
    use super::*;

    /// Helper: build a NavMenuItem with minimal fields.
    fn item(id: &str, parent: Option<&str>, pos: u32, title: &str) -> NavMenuItem {
        NavMenuItem {
            item_id: id.to_string(),
            parent_id: parent.map(|p| p.to_string()),
            title: title.to_string(),
            url: format!("/{}", id),
            description: None,
            icon: None,
            css_classes: vec![],
            position: pos,
            category: None,
            badge: None,
            visibility: None,
        }
    }

    // ── build_tree tests ────────────────────────────────────────────

    #[test]
    fn empty_items_produce_empty_tree() {
        let tree = build_tree(&[]);
        assert!(tree.is_empty());
    }

    #[test]
    fn flat_items_all_top_level() {
        let items = vec![
            item("b", None, 2, "Second"),
            item("a", None, 1, "First"),
            item("c", None, 3, "Third"),
        ];
        let tree = build_tree(&items);
        assert_eq!(tree.len(), 3);
        // Sorted by position
        assert_eq!(tree[0].item.title, "First");
        assert_eq!(tree[1].item.title, "Second");
        assert_eq!(tree[2].item.title, "Third");
        // No children
        for node in &tree {
            assert!(node.children.is_empty());
        }
    }

    #[test]
    fn two_level_hierarchy() {
        let items = vec![
            item("root", None, 0, "Root"),
            item("child-b", Some("root"), 2, "Child B"),
            item("child-a", Some("root"), 1, "Child A"),
        ];
        let tree = build_tree(&items);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].item.item_id, "root");
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].item.title, "Child A");
        assert_eq!(tree[0].children[1].item.title, "Child B");
    }

    #[test]
    fn three_level_hierarchy() {
        let items = vec![
            item("top", None, 0, "Top"),
            item("mid", Some("top"), 0, "Mid"),
            item("leaf", Some("mid"), 0, "Leaf"),
        ];
        let tree = build_tree(&items);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].item.item_id, "mid");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].item.item_id, "leaf");
        assert!(tree[0].children[0].children[0].children.is_empty());
    }

    #[test]
    fn multiple_top_level_with_children() {
        let items = vec![
            item("cat-a", None, 1, "Category A"),
            item("cat-b", None, 2, "Category B"),
            item("a1", Some("cat-a"), 1, "A-1"),
            item("a2", Some("cat-a"), 2, "A-2"),
            item("b1", Some("cat-b"), 1, "B-1"),
        ];
        let tree = build_tree(&items);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[1].children.len(), 1);
    }

    #[test]
    fn orphan_items_are_excluded() {
        // Items whose parent_id references a nonexistent parent
        let items = vec![
            item("root", None, 0, "Root"),
            item("orphan", Some("nonexistent"), 0, "Orphan"),
        ];
        let tree = build_tree(&items);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].item.item_id, "root");
        assert!(tree[0].children.is_empty());
    }

    // ── Serialization round-trip tests ──────────────────────────────

    #[test]
    fn nav_menu_round_trip_json() {
        let menu = NavMenu {
            location: "header".to_string(),
            items: vec![
                item("home", None, 0, "Home"),
                item("about", None, 1, "About"),
                item("sub", Some("about"), 0, "Sub Page"),
            ],
        };
        let json = serde_json::to_vec(&menu).expect("serialize");
        let restored: NavMenu = serde_json::from_slice(&json).expect("deserialize");

        assert_eq!(restored.location, "header");
        assert_eq!(restored.items.len(), 3);
        assert_eq!(restored.items[0].title, "Home");
        assert_eq!(restored.items[2].parent_id.as_deref(), Some("about"));
    }

    #[test]
    fn nav_menu_item_optional_fields_default() {
        let json = r#"{
            "item_id": "test",
            "title": "Test",
            "url": "/test"
        }"#;
        let item: NavMenuItem = serde_json::from_str(json).expect("deserialize");
        assert!(item.parent_id.is_none());
        assert!(item.description.is_none());
        assert!(item.icon.is_none());
        assert!(item.css_classes.is_empty());
        assert_eq!(item.position, 0);
        assert!(item.category.is_none());
        assert!(item.badge.is_none());
    }

    #[test]
    fn nav_menu_item_all_fields() {
        let item = NavMenuItem {
            item_id: "x".into(),
            parent_id: Some("p".into()),
            title: "Full".into(),
            url: "/full".into(),
            description: Some("A description".into()),
            icon: Some("star".into()),
            css_classes: vec!["highlight".into(), "featured".into()],
            position: 5,
            category: Some("Tools".into()),
            badge: Some("New".into()),
            visibility: None,
        };
        let json = serde_json::to_vec(&item).expect("serialize");
        let restored: NavMenuItem = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(restored.item_id, "x");
        assert_eq!(restored.parent_id.as_deref(), Some("p"));
        assert_eq!(restored.description.as_deref(), Some("A description"));
        assert_eq!(restored.icon.as_deref(), Some("star"));
        assert_eq!(restored.css_classes, vec!["highlight", "featured"]);
        assert_eq!(restored.position, 5);
        assert_eq!(restored.category.as_deref(), Some("Tools"));
        assert_eq!(restored.badge.as_deref(), Some("New"));
    }

    #[test]
    fn nav_menu_item_visibility_defaults_to_none() {
        // Existing JSON without visibility field should deserialize with None
        let json = r#"{
            "item_id": "test",
            "title": "Test",
            "url": "/test"
        }"#;
        let item: NavMenuItem = serde_json::from_str(json).expect("deserialize");
        assert!(item.visibility.is_none());
    }

    #[test]
    fn nav_menu_item_visibility_round_trip() {
        let mut item = NavMenuItem {
            item_id: "v".into(),
            parent_id: None,
            title: "Vis".into(),
            url: "/vis".into(),
            description: None,
            icon: None,
            css_classes: vec![],
            position: 0,
            category: None,
            badge: None,
            visibility: Some("authenticated".into()),
        };
        let json = serde_json::to_vec(&item).expect("serialize");
        let restored: NavMenuItem = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(restored.visibility.as_deref(), Some("authenticated"));

        // None should be omitted from serialization (skip_serializing_if)
        item.visibility = None;
        let json2 = serde_json::to_string(&item).expect("serialize");
        assert!(!json2.contains("visibility"));
    }

    #[test]
    fn tombstone_is_distinct_from_valid_payload() {
        let menu = NavMenu {
            location: "test".to_string(),
            items: vec![],
        };
        let payload = serde_json::to_vec(&menu).expect("serialize");
        // A valid JSON payload should never match the tombstone bytes
        assert_ne!(payload.as_slice(), TOMBSTONE);
    }

    // ── list_structures tests ──────────────────────────────────────

    #[test]
    fn list_structures_returns_only_structure_prefixed() {
        let tmp = tempfile::TempDir::new().expect("tmpdir");
        let wal = tmp.path().join("events.wal");
        let snap = tmp.path().join("snapshot.bin");
        let mut journal =
            ForgeJournal::open(&wal, &snap, luperiq_forge::DurabilityMode::Sync).expect("open");

        // Save a primary menu
        let primary = NavMenu {
            location: "primary".to_string(),
            items: vec![item("home", None, 0, "Home")],
        };
        save_menu(&mut journal, &primary).expect("save primary");

        // Save a structure
        let focused = NavMenu {
            location: "structure:focused".to_string(),
            items: vec![item("a", None, 0, "A"), item("b", None, 1, "B")],
        };
        save_menu(&mut journal, &focused).expect("save focused");

        // Save another structure
        let full = NavMenu {
            location: "structure:full".to_string(),
            items: vec![item("x", None, 0, "X")],
        };
        save_menu(&mut journal, &full).expect("save full");

        let structures = list_structures(&journal).expect("list");
        assert_eq!(structures.len(), 2);
        let locs: Vec<&str> = structures.iter().map(|s| s.location.as_str()).collect();
        assert!(locs.contains(&"structure:focused"));
        assert!(locs.contains(&"structure:full"));
        // Primary should NOT be in structures
        assert!(!locs.contains(&"primary"));
    }
}
