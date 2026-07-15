use axum::response::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PromoRequest {
    code: String,
}

#[derive(Serialize)]
pub struct PromoData {
    code: String,
    #[serde(rename = "type")]
    kind: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum PromoResponse {
    Ok { ok: bool, data: PromoData },
    Err { ok: bool, error: &'static str },
}

struct PromoCode {
    kind: &'static str,
    description: &'static str,
}

fn lookup(code: &str) -> Option<PromoCode> {
    match code {
        "LAUNCH2026" => Some(PromoCode {
            kind: "dfy_discount",
            description: "$200 off done-for-you AI content setup — applied at checkout.",
        }),
        "PESTPRO" => Some(PromoCode {
            kind: "subscription_discount",
            description: "One month free on any paid subscription — applied at checkout.",
        }),
        "NPMA2026" => Some(PromoCode {
            kind: "dfy_discount",
            description: "15% off done-for-you AI content setup — applied at checkout.",
        }),
        _ => None,
    }
}

pub async fn validate_handler(Json(req): Json<PromoRequest>) -> Json<serde_json::Value> {
    let code = req.code.trim().to_uppercase();
    match lookup(&code) {
        Some(promo) => Json(serde_json::json!({
            "ok": true,
            "data": {
                "code": code,
                "type": promo.kind,
                "description": promo.description
            }
        })),
        None => Json(serde_json::json!({
            "ok": false,
            "error": "Invalid promo code."
        })),
    }
}
