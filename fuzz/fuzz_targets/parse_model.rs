#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = home_rule_core::parse_scenario(text);
        if let Ok(rules) = home_rule_ha::parse(text) {
            let mut s =
                home_rule_core::parse_scenario(include_str!("../../examples/alarm.json")).unwrap();
            s.limits.max_steps = 100;
            let _ = home_rule_core::simulate(&s, &rules);
        }
    }
});
