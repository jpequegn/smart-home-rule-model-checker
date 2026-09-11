use home_rule_core::*;
use proptest::prelude::*;
proptest! {
 #![proptest_config(ProptestConfig::with_cases(64))]
 #[test]
 fn replay_is_deterministic(times in prop::collection::vec(0u64..12,0..15)){
   let mut s=parse_scenario(include_str!("../../../examples/alarm.json")).unwrap();
   let mut times=times;times.sort();
   s.events=times.into_iter().enumerate().map(|(i,at)|Event{at,input:Input::Set{entity:"binary_sensor.motion".into(),value:if i%2==0{"on"}else{"off"}.into()}}).collect();
   let r=home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap();
   prop_assert_eq!(simulate(&s,&r).unwrap(),simulate(&s,&r).unwrap());
 }
 #[test]
 fn minimization_preserves_violation(extra in 0u64..3){
   let mut s=parse_scenario(include_str!("../../../examples/alarm.json")).unwrap();
   s.events.insert(0,Event{at:0,input:Input::Set{entity:"input_boolean.alarm".into(),value:"off".into()}});
   s.limits.horizon+=extra;
   let r=home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap();
   let m=minimize(&s,&r).unwrap();prop_assert_eq!(&m.check.status,"violation");prop_assert!(m.deletion_minimal);
   for i in 0..m.scenario.events.len(){let mut reduced=m.scenario.clone();reduced.events.remove(i);prop_assert_ne!(check(&reduced,&r).unwrap().status,"violation");}
 }
 #[test]
 fn malformed_yaml_never_panics(input in ".{0,1000}"){
   let _=home_rule_ha::parse(&input);
 }
 #[test]
 fn expanding_alphabet_preserves_witness(gap in 0u64..6){
   let mut s=parse_scenario(include_str!("../../../examples/alarm.json")).unwrap();
   s.gaps=vec![gap];s.limits.search_depth=1;s.alphabet.truncate(1);
   s.invariants=vec![Invariant::Never{id:"light".into(),conditions:vec![Predicate::State{entity:"light.hall".into(),value:"on".into()}]}];
   let r=home_rule_ha::parse(include_str!("../../../examples/alarm.yaml")).unwrap();
   prop_assert_eq!(verify(&s,&r).unwrap().status,"counterexample");
   s.alphabet.insert(0,Input::Restart);
   prop_assert_eq!(verify(&s,&r).unwrap().status,"counterexample");
 }
}
