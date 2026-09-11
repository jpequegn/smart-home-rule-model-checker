use crate::parse;
use home_rule_core::*;
use serde_json::{Value, json};

pub fn analyze(operation: &str, yaml: &str, scenario_json: &str) -> Result<Value> {
    let s = parse_scenario(scenario_json)?;
    let rules = parse(yaml)?;
    validate(&s, &rules)?;
    let value = match operation {
        "lint" => serde_json::to_value(lint(&s, &rules)?),
        "simulate" => serde_json::to_value(check(&s, &rules)?),
        "verify" => serde_json::to_value(verify(&s, &rules)?),
        "minimize" => serde_json::to_value(minimize(&s, &rules)?),
        "report" => Ok(
            json!({"lint":lint(&s,&rules)?,"replay":check(&s,&rules)?,"verification":verify(&s,&rules)?}),
        ),
        _ => return Err("unknown operation".into()),
    };
    value.map_err(|e| e.to_string())
}
pub fn diff(before: &str, after: &str, scenario_json: &str) -> Result<Value> {
    let s = parse_scenario(scenario_json)?;
    let a = parse(before)?;
    let b = parse(after)?;
    validate(&s, &a)?;
    validate(&s, &b)?;
    if s.invariants.is_empty() {
        return Err("diff requires invariants".into());
    }
    let mut comparisons = vec![];
    for invariant in &s.invariants {
        let mut one = s.clone();
        one.invariants = vec![invariant.clone()];
        let old = verify(&one, &a)?;
        let new = verify(&one, &b)?;
        let classification = match (old.status.as_str(), new.status.as_str()) {
            ("no_counterexample_within_bounds", "counterexample") => {
                "new_counterexample_within_bounds"
            }
            ("counterexample", "no_counterexample_within_bounds") => {
                "counterexample_not_found_after"
            }
            ("inconclusive", _) | (_, "inconclusive") => "inconclusive_comparison",
            _ => "unchanged_outcome",
        };
        comparisons.push(json!({"invariant":invariant.id(),"classification":classification,"before":old,"after":new}));
    }
    Ok(
        json!({"comparisons":comparisons,"scope":"same declared finite event alphabet, depth and timing bounds; not unbounded equivalence"}),
    )
}
pub fn outcome(value: &Value) -> &str {
    if let Some(items) = value.get("comparisons").and_then(Value::as_array) {
        if items
            .iter()
            .any(|i| i["classification"] == "new_counterexample_within_bounds")
        {
            return "counterexample";
        }
        if items
            .iter()
            .any(|i| i["classification"] == "inconclusive_comparison")
        {
            return "inconclusive";
        }
    }
    if value
        .get("findings")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|i| i["certainty"] == "unknown"))
    {
        return "unknown";
    }
    if let Some(replay) = value.get("replay") {
        let result = outcome(replay);
        if matches!(result, "violation" | "unknown" | "incomplete") {
            return result;
        }
    }
    if let Some(v) = value.get("status").and_then(Value::as_str) {
        return v;
    }
    if let Some(v) = value.get("verification") {
        return outcome(v);
    }
    if let Some(v) = value.get("check") {
        return outcome(v);
    }
    "review"
}
pub fn exit_code(v: &Value) -> u8 {
    match outcome(v) {
        "violation" | "counterexample" => 1,
        "unknown" | "incomplete" | "inconclusive" => 3,
        _ => 0,
    }
}
fn xml(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn render(value: &Value, format: &str) -> Result<String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    match format {
        "json" => Ok(json + "\n"),
        "markdown" => Ok(format!(
            "# Home rule analysis\n\nOutcome: {}\n\nOffline bounded model only. Lint findings are potential hazards; UNKNOWN is not safe.\n\n<details><summary>Source-linked evidence and exploration bounds</summary>\n\n<pre>{}</pre>\n</details>\n",
            outcome(value),
            xml(&json)
        )),
        "junit" => {
            let code = exit_code(value);
            let child = match code {
                1 => format!(
                    "<failure message=\"modeled counterexample\">{}</failure>",
                    xml(&json)
                ),
                3 => format!("<skipped message=\"inconclusive\">{}</skipped>", xml(&json)),
                _ => format!("<system-out>{}</system-out>", xml(&json)),
            };
            Ok(format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"homecheck\" tests=\"1\" failures=\"{}\" skipped=\"{}\"><testcase name=\"bounded analysis\">{child}</testcase></testsuite>\n",
                u8::from(code == 1),
                u8::from(code == 3)
            ))
        }
        _ => Err("format must be json, markdown or junit".into()),
    }
}
