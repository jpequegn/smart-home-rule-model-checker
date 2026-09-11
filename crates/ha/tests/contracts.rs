use home_rule_core::*;
fn scenario() -> Scenario {
    serde_json::from_str(include_str!("../../../examples/alarm.json")).unwrap()
}
#[test]
fn example_contract() {
    validate(
        &scenario(),
        &home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap(),
    )
    .unwrap();
}
#[test]
fn reference_and_budget_checks() {
    let mut s = scenario();
    s.events[0].input = Input::Set {
        entity: "light.missing".into(),
        value: "on".into(),
    };
    assert!(validate(&s, &[]).is_err());
    let mut s = scenario();
    s.limits.max_steps = 10001;
    assert!(validate(&s, &[]).is_err());
    let mut s = scenario();
    s.events.reverse();
    assert!(validate(&s, &[]).is_err());
}
#[test]
fn json_unknown_fields_rejected() {
    let s = include_str!("../../../examples/alarm.json").replace("\"version\": 1", "\"typo\": 1");
    assert!(serde_json::from_str::<Scenario>(&s).is_err());
}
