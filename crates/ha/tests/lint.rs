use home_rule_core::*;
fn s() -> Scenario {
    serde_json::from_str(include_str!("../../../examples/alarm.json")).unwrap()
}
fn rules() -> Vec<Automation> {
    home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap()
}
#[test]
fn conflicting_writes_have_sources() {
    let l = lint(&s(), &rules()).unwrap();
    let f = l
        .findings
        .iter()
        .find(|f| f.code == "opposing_writes")
        .unwrap();
    assert_eq!(f.certainty, "potential");
    assert_eq!(f.sources.len(), 2);
}
#[test]
fn benign_single_rule() {
    let mut r = rules();
    r.truncate(1);
    assert!(lint(&s(), &r).unwrap().findings.is_empty());
}
#[test]
fn self_cycle() {
    let mut r = rules();
    r.truncate(1);
    r[0].triggers[0].entity = "light.hall".into();
    r[0].triggers[0].to = None;
    r[0].actions[0].effect = Effect::Toggle {
        entity: "light.hall".into(),
    };
    assert_eq!(lint(&s(), &r).unwrap().components, vec![vec!["motion-on"]]);
}
#[test]
fn impossible_condition() {
    let mut r = rules();
    r.truncate(1);
    r[0].conditions = vec![Predicate::State {
        entity: "binary_sensor.motion".into(),
        value: "off".into(),
    }];
    assert!(
        lint(&s(), &r)
            .unwrap()
            .findings
            .iter()
            .any(|f| f.code == "unreachable_conditions")
    );
}
