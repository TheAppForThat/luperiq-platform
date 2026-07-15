use rusqlite::Connection;
use serde_json::Value;

fn directory_db_path() -> String {
    std::env::var("LUPERIQ_DIRECTORY_DB")
        .unwrap_or_else(|_| "/home/dave/directory/directory.sqlite".to_string())
}

/// Enrich wizard_answers with directory data when a company slug is present.
/// Best-effort — never blocks provisioning on DB failure.
pub fn enrich_wizard_answers_from_directory(mut wizard_answers: Value) -> Value {
    let obj = match wizard_answers.as_object_mut() {
        Some(o) => o,
        None => return wizard_answers,
    };

    let company_slug = match obj
        .remove("_directory_company_slug")
        .and_then(|v| v.as_str().map(str::to_string))
    {
        Some(s) if !s.is_empty() => s,
        _ => return wizard_answers,
    };
    let state = obj
        .remove("_directory_state")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "TX".to_string());

    let db_path = directory_db_path();
    let conn = match Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[dir-enrich] Could not open directory DB at {db_path}: {e}");
            return wizard_answers;
        }
    };

    struct CompanyRow {
        id: String,
        entity_name: String,
        dba: Option<String>,
        phone: Option<String>,
        website: Option<String>,
        address: Option<String>,
        city: Option<String>,
        tagline: Option<String>,
        crawl_description: Option<String>,
        hours: Option<String>,
        pest_categories: Option<String>,
    }

    let row_result = conn.query_row(
        "SELECT id, entity_name, dba, phone, website, address, city,
                tagline, crawl_description, hours, pest_categories_decoded
         FROM companies
         WHERE company_slug = ?1 AND state_abbr = ?2
         LIMIT 1",
        rusqlite::params![company_slug, state.to_uppercase()],
        |row| {
            Ok(CompanyRow {
                id: row.get(0)?,
                entity_name: row.get(1)?,
                dba: row.get(2)?,
                phone: row.get(3)?,
                website: row.get(4)?,
                address: row.get(5)?,
                city: row.get(6)?,
                tagline: row.get(7)?,
                crawl_description: row.get(8)?,
                hours: row.get(9)?,
                pest_categories: row.get(10)?,
            })
        },
    );

    let company = match row_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[dir-enrich] Company not found slug={} state={}: {}",
                company_slug, state, e
            );
            return wizard_answers;
        }
    };

    eprintln!(
        "[dir-enrich] Enriching from directory: slug={} id={} name={}",
        company_slug, company.id, company.entity_name
    );

    let obj = wizard_answers.as_object_mut().unwrap();

    let mut set_if_empty = |key: &str, val: Option<String>| {
        if let Some(v) = val.filter(|s| !s.trim().is_empty()) {
            let existing = obj.get(key).and_then(|x| x.as_str()).unwrap_or("").trim();
            if existing.is_empty() {
                obj.insert(key.to_string(), Value::String(v));
            }
        }
    };

    let display_name = company.dba.clone().or_else(|| Some(company.entity_name.clone()));
    set_if_empty("business_name", display_name);
    set_if_empty("phone", company.phone);
    set_if_empty("website_url", company.website);
    set_if_empty("address", company.address);
    set_if_empty("city", company.city);
    set_if_empty("tagline", company.tagline);
    set_if_empty("enrichment_description", company.crawl_description);
    set_if_empty("hours", company.hours);
    set_if_empty("pest_categories", company.pest_categories);

    let has_services = obj
        .get("services")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if !has_services {
        let service_names = load_service_names(&conn, &company.id);
        if !service_names.is_empty() {
            obj.insert(
                "services".to_string(),
                Value::Array(service_names.into_iter().map(Value::String).collect()),
            );
        }
    }

    obj.insert("_from_directory_claim".to_string(), Value::Bool(true));

    wizard_answers
}

fn load_service_names(conn: &Connection, company_id: &str) -> Vec<String> {
    let blocks_json: Option<String> = conn
        .query_row(
            "SELECT blocks_json FROM company_pages
             WHERE company_id = ?1 AND page_slug = 'services'",
            rusqlite::params![company_id],
            |row| row.get::<_, String>(0),
        )
        .ok();

    let json_str = match blocks_json {
        Some(s) if !s.trim().is_empty() && s.trim() != "[]" => s,
        _ => return Vec::new(),
    };

    let blocks: Vec<Value> = serde_json::from_str(&json_str).unwrap_or_default();
    blocks
        .into_iter()
        .filter_map(|b| {
            let block_type = b.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if block_type == "service" {
                b.get("heading")
                    .and_then(|h| h.as_str())
                    .filter(|h| !h.trim().is_empty())
                    .map(str::to_string)
            } else {
                None
            }
        })
        .take(8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_op_when_no_slug() {
        let input = json!({"email": "test@example.com", "phone": "555-1234"});
        let output = enrich_wizard_answers_from_directory(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn strips_meta_fields_when_db_unavailable() {
        // SAFETY: test-only env mutation, single-threaded test runner.
        unsafe { std::env::set_var("LUPERIQ_DIRECTORY_DB", "/tmp/nonexistent-dir-test.sqlite") };
        let input = json!({
            "email": "test@example.com",
            "_directory_company_slug": "test-co",
            "_directory_state": "TX"
        });
        let output = enrich_wizard_answers_from_directory(input);
        let obj = output.as_object().unwrap();
        assert!(!obj.contains_key("_directory_company_slug"));
        assert!(!obj.contains_key("_directory_state"));
    }

    #[test]
    fn existing_phone_not_overwritten() {
        let mut obj = serde_json::Map::new();
        obj.insert("phone".to_string(), Value::String("555-original".to_string()));
        let wa = Value::Object(obj);
        assert_eq!(wa.get("phone").and_then(|v| v.as_str()), Some("555-original"));
    }
}
