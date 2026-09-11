use home_rule_core::*;
use serde_json::Value;
#[test]
fn golden_corpus() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../examples/corpus.json")).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert!(cases.len() >= 25);
    for c in cases {
        let s = parse_scenario(&c["scenario"].to_string()).unwrap();
        let r = home_rule_ha::parse(c["yaml"].as_str().unwrap()).unwrap();
        let check = check(&s, &r).unwrap();
        assert_eq!(check.status, c["expected"]["status"], "{}", c["name"]);
        let times: Vec<_> = check
            .simulation
            .trace
            .iter()
            .filter(|t| t.kind == "action")
            .map(|t| t.at)
            .collect();
        assert_eq!(
            serde_json::to_value(times).unwrap(),
            c["expected"]["action_times"],
            "{}",
            c["name"]
        );
        let l = lint(&s, &r).unwrap();
        for code in c["expected"]["lint_codes"].as_array().unwrap() {
            assert!(
                l.findings.iter().any(|f| f.code == code.as_str().unwrap()),
                "{} missing {code}",
                c["name"]
            );
        }
    }
}
#[test]
fn targeted_mutations() {
    let yaml = include_str!("../../../examples/alarm.yaml");
    let s = parse_scenario(include_str!("../../../examples/alarm.json")).unwrap();
    let safe=yaml.replace("    - delay: 3","    - delay: 3\n    - condition: state\n      entity_id: input_boolean.alarm\n      state: \"off\"");
    assert_eq!(
        check(&s, &home_rule_ha::parse(&safe).unwrap())
            .unwrap()
            .status,
        "no_violation_in_trace"
    );
    assert_eq!(
        check(
            &s,
            &home_rule_ha::parse(&safe.replace("state: \"off\"", "state: \"on\"")).unwrap()
        )
        .unwrap()
        .status,
        "violation"
    );
    assert_eq!(
        check(
            &s,
            &home_rule_ha::parse(&yaml.replace("delay: 3", "delay: 0")).unwrap()
        )
        .unwrap()
        .status,
        "no_violation_in_trace"
    );
    assert!(
        validate(
            &s,
            &home_rule_ha::parse(
                &yaml.replace("entity_id: light.hall", "entity_id: light.missing")
            )
            .unwrap()
        )
        .is_err()
    );
}
