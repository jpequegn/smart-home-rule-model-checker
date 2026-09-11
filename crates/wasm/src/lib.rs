//! Coarse JSON/text adapter; no network or browser privileges.
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn analyze(operation: &str, yaml: &str, scenario: &str) -> Result<String, JsValue> {
    let value =
        home_rule_ha::analyze(operation, yaml, scenario).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&value).map_err(|e| JsValue::from_str(&e.to_string()))
}
