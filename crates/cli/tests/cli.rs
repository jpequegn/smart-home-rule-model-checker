use std::process::Command;
fn run(op: &str, format: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_homecheck"))
        .args([
            op,
            "--rules",
            "../../examples/alarm.yaml",
            "--scenario",
            "../../examples/alarm.json",
            "--format",
            format,
        ])
        .output()
        .unwrap()
}
#[test]
fn replay_and_reports() {
    let r = run("simulate", "json");
    assert_eq!(r.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(v["status"], "violation");
    let md = run("report", "markdown");
    assert!(
        String::from_utf8(md.stdout)
            .unwrap()
            .starts_with("# Home rule analysis")
    );
    let xml = run("verify", "junit");
    assert!(
        String::from_utf8(xml.stdout)
            .unwrap()
            .contains("failures=\"1\"")
    );
}
#[test]
fn refuses_overwrite() {
    let p = std::env::temp_dir().join(format!("homecheck-existing-{}", std::process::id()));
    std::fs::write(&p, "keep").unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_homecheck"))
        .args([
            "lint",
            "--rules",
            "../../examples/alarm.yaml",
            "--scenario",
            "../../examples/alarm.json",
            "--out",
        ])
        .arg(&p)
        .output()
        .unwrap();
    assert_eq!(r.status.code(), Some(2));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "keep");
    std::fs::remove_file(p).unwrap();
}
#[test]
fn diff_command() {
    let r = Command::new(env!("CARGO_BIN_EXE_homecheck"))
        .args([
            "diff",
            "--before",
            "../../examples/alarm.yaml",
            "--rules",
            "../../examples/alarm.yaml",
            "--scenario",
            "../../examples/alarm.json",
        ])
        .output()
        .unwrap();
    assert!(r.status.success());
    let v: serde_json::Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(v["comparisons"][0]["classification"], "unchanged_outcome");
}
