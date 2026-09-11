//! Coarse JSON/text adapter; no network or browser privileges.
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn analyze(operation: &str, yaml: &str, scenario: &str) -> Result<String, JsValue> {
    let value =
        home_rule_ha::analyze(operation, yaml, scenario).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&value).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;
    #[wasm_bindgen_test]
    fn golden_corpus_in_wasm() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/corpus.json")).unwrap();
        for c in corpus["cases"].as_array().unwrap() {
            let result = analyze(
                "simulate",
                c["yaml"].as_str().unwrap(),
                &c["scenario"].to_string(),
            )
            .unwrap();
            let value: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(value["status"], c["expected"]["status"], "{}", c["name"]);
        }
    }
}
