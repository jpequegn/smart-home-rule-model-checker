use home_rule_core::*;
fn s() -> Scenario {
    serde_json::from_str(include_str!("../../../examples/alarm.json")).unwrap()
}
fn r(mode: &str) -> Vec<Automation> {
    home_rule_ha::parse(&format!("- id: test\n  mode: {mode}\n  max: 3\n  triggers:\n    - trigger: state\n      entity_id: binary_sensor.motion\n      to: 'on'\n  actions:\n    - delay: 3\n    - action: light.toggle\n      target:\n        entity_id: light.hall\n")).unwrap()
}
fn repeat() -> Scenario {
    let mut s = s();
    s.events = vec![ev(0, "on"), ev(1, "off"), ev(2, "on")];
    s
}
fn ev(at: u64, value: &str) -> Event {
    Event {
        at,
        input: Input::Set {
            entity: "binary_sensor.motion".into(),
            value: value.into(),
        },
    }
}
fn writes(sim: &Simulation) -> Vec<u64> {
    sim.trace
        .iter()
        .filter(|t| t.kind == "action")
        .map(|t| t.at)
        .collect()
}
#[test]
fn modes() {
    for (mode, expected) in [
        ("single", vec![3]),
        ("restart", vec![5]),
        ("queued", vec![3, 6]),
        ("parallel", vec![3, 5]),
    ] {
        assert_eq!(
            writes(&simulate(&repeat(), &r(mode)).unwrap()),
            expected,
            "{mode}"
        );
    }
}
#[test]
fn restart_cancels_delays() {
    let mut s = repeat();
    s.events = vec![
        ev(0, "on"),
        Event {
            at: 1,
            input: Input::Restart,
        },
    ];
    assert!(writes(&simulate(&s, &r("queued")).unwrap()).is_empty());
}
#[test]
fn unknown_and_stale_conditions() {
    let mut s = repeat();
    s.entities
        .get_mut("input_boolean.alarm")
        .unwrap()
        .stale_after = Some(0);
    let mut rules = r("single");
    rules[0].conditions = vec![Predicate::State {
        entity: "input_boolean.alarm".into(),
        value: "off".into(),
    }];
    let sim = simulate(&s, &rules).unwrap();
    assert!(!sim.unknown.is_empty());
    assert_eq!(writes(&sim), vec![3]);
}
#[test]
fn determinism_and_bounds() {
    let s = repeat();
    assert_eq!(
        simulate(&s, &r("parallel")).unwrap(),
        simulate(&s, &r("parallel")).unwrap()
    );
    let mut s = s;
    s.limits.max_steps = 1;
    assert!(!simulate(&s, &r("single")).unwrap().incomplete.is_empty());
}
#[test]
fn wait_timeout_and_success() {
    let mut rules = r("single");
    rules[0].actions[0].effect = Effect::Wait {
        trigger: Trigger {
            entity: "binary_sensor.motion".into(),
            from: None,
            to: Some("off".into()),
            source: Source::default(),
            unknown: None,
        },
        timeout: 4,
        continue_on_timeout: false,
    };
    assert_eq!(writes(&simulate(&repeat(), &rules).unwrap()), vec![1]);
    let mut s = repeat();
    s.events.truncate(1);
    assert!(writes(&simulate(&s, &rules).unwrap()).is_empty());
}
#[test]
fn queued_conditions_checked_at_admission() {
    let mut rules = r("queued");
    rules[0].conditions = vec![Predicate::State {
        entity: "light.hall".into(),
        value: "off".into(),
    }];
    assert_eq!(writes(&simulate(&repeat(), &rules).unwrap()), vec![3, 6]);
}
