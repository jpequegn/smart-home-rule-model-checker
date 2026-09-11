use home_rule_core::*;
fn s() -> Scenario {
    serde_json::from_str(include_str!("../../../examples/alarm.json")).unwrap()
}
fn r() -> Vec<Automation> {
    home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap()
}
#[test]
fn witness_and_minimality() {
    let s = s();
    let c = check(&s, &r()).unwrap();
    assert_eq!(c.status, "violation");
    assert_eq!(c.violations[0].source.as_ref().unwrap().line, 18);
    let m = minimize(&s, &r()).unwrap();
    assert!(m.deletion_minimal);
    for i in 0..m.scenario.events.len() {
        let mut reduced = m.scenario.clone();
        reduced.events.remove(i);
        assert_ne!(check(&reduced, &r()).unwrap().status, "violation");
    }
}
#[test]
fn breadth_first_search() {
    let v = verify(&s(), &r()).unwrap();
    assert_eq!(v.status, "counterexample");
    assert!(v.shortest_by_event_count);
    assert_eq!(v.witness.unwrap().events.len(), 3);
}
#[test]
fn resource_exhaustion_never_passes() {
    let mut s = s();
    s.limits.max_cases = 1;
    assert_eq!(verify(&s, &r()).unwrap().status, "inconclusive");
}
#[test]
fn unsupported_never_passes() {
    let mut r = r();
    r[0].actions[0].effect = Effect::Unknown {
        reason: "test".into(),
    };
    assert_eq!(check(&s(), &r).unwrap().status, "unknown");
    assert_eq!(verify(&s(), &r).unwrap().status, "inconclusive");
}
#[test]
fn temporal_deadline() {
    let mut s = s();
    s.events.clear();
    s.invariants = vec![Invariant::Response {
        id: "response".into(),
        when: Predicate::State {
            entity: "input_boolean.alarm".into(),
            value: "off".into(),
        },
        then: Predicate::State {
            entity: "light.hall".into(),
            value: "on".into(),
        },
        within: 5,
    }];
    assert_eq!(check(&s, &[]).unwrap().status, "violation");
    s.limits.horizon = 3;
    assert_eq!(check(&s, &[]).unwrap().status, "incomplete");
}
#[test]
fn guard_uses_pre_action_state() {
    let mut s = s();
    s.invariants = vec![Invariant::Guard {
        id: "pre".into(),
        entity: "light.hall".into(),
        value: "on".into(),
        requires: vec![Predicate::State {
            entity: "light.hall".into(),
            value: "off".into(),
        }],
    }];
    s.events.truncate(1);
    assert_eq!(check(&s, &r()).unwrap().status, "no_violation_in_trace");
}
