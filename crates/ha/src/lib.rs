use home_rule_core::*;
mod api;
pub use api::*;
use std::collections::BTreeMap;
use yaml_rust2::parser::{Event as YEvent, Parser};
use yaml_rust2::scanner::Marker;

#[derive(Debug)]
enum Value {
    Scalar(String),
    Seq(Vec<Node>),
    Map(BTreeMap<String, Node>),
}
#[derive(Debug)]
struct Node {
    value: Value,
    source: Source,
}
impl Node {
    fn map(&self) -> Result<&BTreeMap<String, Node>> {
        if let Value::Map(v) = &self.value {
            Ok(v)
        } else {
            Err(self.err("expected mapping"))
        }
    }
    fn seq(&self) -> Result<&[Node]> {
        if let Value::Seq(v) = &self.value {
            Ok(v)
        } else {
            Err(self.err("expected sequence"))
        }
    }
    fn text(&self) -> Result<String> {
        if let Value::Scalar(v) = &self.value {
            Ok(v.clone())
        } else {
            Err(self.err("expected literal scalar"))
        }
    }
    fn get(&self, k: &str) -> Result<&Node> {
        self.map()?
            .get(k)
            .ok_or_else(|| self.err(&format!("missing {k}")))
    }
    fn opt(&self, k: &str) -> Option<&Node> {
        self.map().ok()?.get(k)
    }
    fn str(&self, k: &str) -> Result<String> {
        self.get(k)?.text()
    }
    fn extra(&self, allowed: &[&str]) -> Result<bool> {
        Ok(self.map()?.keys().any(|k| !allowed.contains(&k.as_str())))
    }
    fn err(&self, msg: &str) -> String {
        format!("{}:{}: {msg}", self.source.line, self.source.column)
    }
}
fn take(p: &mut Parser<std::str::Chars<'_>>) -> Result<(YEvent, Marker)> {
    p.next_token().map_err(|e| e.to_string())
}
fn node(p: &mut Parser<std::str::Chars<'_>>, depth: usize, count: &mut usize) -> Result<Node> {
    if depth > 24 {
        *count = usize::MAX;
        return Err("YAML depth exceeds 24".into());
    }
    *count += 1;
    if *count > 10000 {
        return Err("YAML node limit".into());
    }
    let (ev, m) = take(p)?;
    let source = Source {
        line: m.line(),
        column: m.col() + 1,
    };
    let value = match ev {
        YEvent::Scalar(s, _, 0, None) => Value::Scalar(s),
        YEvent::SequenceStart(0, None) => {
            let mut v = Vec::new();
            while !matches!(p.peek().map_err(|e| e.to_string())?.0, YEvent::SequenceEnd) {
                v.push(node(p, depth + 1, count)?);
            }
            take(p)?;
            Value::Seq(v)
        }
        YEvent::MappingStart(0, None) => {
            let mut v = BTreeMap::new();
            while !matches!(p.peek().map_err(|e| e.to_string())?.0, YEvent::MappingEnd) {
                let k = node(p, depth + 1, count)?.text()?;
                let val = node(p, depth + 1, count)?;
                if v.insert(k.clone(), val).is_some() {
                    return Err(format!("duplicate YAML key: {k}"));
                }
            }
            take(p)?;
            Value::Map(v)
        }
        _ => {
            return Err(format!(
                "{}:{}: unsupported YAML event (aliases, anchors and tags forbidden)",
                source.line, source.column
            ));
        }
    };
    Ok(Node { value, source })
}
fn literal(s: &str) -> bool {
    !s.contains("{{") && !s.contains("{%") && !s.contains('[')
}
fn trigger(n: &Node) -> Result<Trigger> {
    let mut t = Trigger {
        entity: String::new(),
        from: None,
        to: None,
        source: n.source.clone(),
        unknown: None,
    };
    if n.extra(&["trigger", "platform", "entity_id", "from", "to"])?
        || (n
            .opt("trigger")
            .or(n.opt("platform"))
            .map(Node::text)
            .transpose()?
            .as_deref()
            != Some("state"))
        || (n.opt("trigger").is_some() && n.opt("platform").is_some())
    {
        t.unknown = Some("unsupported trigger".into());
        return Ok(t);
    }
    t.entity = n.str("entity_id")?;
    t.from = n.opt("from").map(Node::text).transpose()?;
    t.to = n.opt("to").map(Node::text).transpose()?;
    if !literal(&t.entity) || t.from.iter().chain(t.to.iter()).any(|v| !literal(v)) {
        t.unknown = Some("dynamic trigger".into());
    }
    Ok(t)
}
fn predicate(n: &Node) -> Result<Predicate> {
    if n.extra(&["condition", "entity_id", "state"])? || n.str("condition")? != "state" {
        return Ok(Predicate::Unknown {
            reason: "unsupported condition".into(),
        });
    }
    let entity = n.str("entity_id")?;
    let value = n.str("state")?;
    if !literal(&entity) || !literal(&value) {
        return Ok(Predicate::Unknown {
            reason: "dynamic condition".into(),
        });
    }
    Ok(Predicate::State { entity, value })
}
fn predicates(n: Option<&Node>) -> Result<Vec<Predicate>> {
    n.map(|n| n.seq()?.iter().map(predicate).collect())
        .unwrap_or(Ok(vec![]))
}
fn seconds(n: &Node) -> Result<u64> {
    let v = n
        .text()?
        .parse::<u64>()
        .map_err(|_| n.err("expected integer seconds"))?;
    if v > 86400 {
        Err(n.err("duration exceeds 86400"))
    } else {
        Ok(v)
    }
}
fn actions(n: &Node) -> Result<Vec<Action>> {
    n.seq()?.iter().map(action).collect()
}
fn action(n: &Node) -> Result<Action> {
    let unknown = || Effect::Unknown {
        reason: "unsupported action, target, or service".into(),
    };
    let effect = if let Some(d) = n.opt("delay") {
        if n.extra(&["delay", "alias"])? {
            unknown()
        } else {
            Effect::Delay {
                seconds: seconds(d)?,
            }
        }
    } else if let Some(c) = n.opt("choose") {
        if n.extra(&["choose", "default", "alias"])? {
            unknown()
        } else {
            let mut branches = vec![];
            for b in c.seq()? {
                if b.extra(&["conditions", "sequence"])? {
                    return Ok(Action {
                        source: n.source.clone(),
                        effect: unknown(),
                    });
                }
                let conditions = predicates(Some(b.get("conditions")?))?;
                branches.push(Branch {
                    conditions,
                    sequence: actions(b.get("sequence")?)?,
                });
            }
            Effect::Choose {
                branches,
                default: n
                    .opt("default")
                    .map(actions)
                    .transpose()?
                    .unwrap_or_default(),
            }
        }
    } else if let Some(w) = n.opt("wait_for_trigger") {
        if n.extra(&[
            "wait_for_trigger",
            "timeout",
            "continue_on_timeout",
            "alias",
        ])? || w.seq()?.len() != 1
        {
            unknown()
        } else {
            let cont = match n
                .opt("continue_on_timeout")
                .map(Node::text)
                .transpose()?
                .as_deref()
            {
                None | Some("true") => true,
                Some("false") => false,
                _ => return Err(n.err("expected boolean continue_on_timeout")),
            };
            Effect::Wait {
                trigger: trigger(&w.seq()?[0])?,
                timeout: seconds(n.get("timeout")?)?,
                continue_on_timeout: cont,
            }
        }
    } else if n.opt("condition").is_some() {
        Effect::Condition {
            predicate: predicate(n)?,
        }
    } else if let Some(service) = n.opt("action").or(n.opt("service")) {
        if n.extra(&["action", "service", "target", "alias"])?
            || (n.opt("action").is_some() && n.opt("service").is_some())
        {
            unknown()
        } else {
            let target = n.get("target")?;
            if target.extra(&["entity_id"])? || target.get("entity_id")?.text().is_err() {
                unknown()
            } else {
                let entity = target.str("entity_id")?;
                let service = service.text()?;
                let parts = service.split_once('.');
                if !literal(&entity)
                    || !matches!(parts, Some(("light" | "switch" | "input_boolean", _)))
                    || !entity.starts_with(&format!("{}.", parts.map(|v| v.0).unwrap_or("")))
                {
                    unknown()
                } else {
                    match parts.map(|v| v.1) {
                        Some("turn_on") => Effect::Set {
                            entity,
                            value: "on".into(),
                        },
                        Some("turn_off") => Effect::Set {
                            entity,
                            value: "off".into(),
                        },
                        Some("toggle") => Effect::Toggle { entity },
                        _ => unknown(),
                    }
                }
            }
        }
    } else {
        unknown()
    };
    Ok(Action {
        source: n.source.clone(),
        effect,
    })
}
pub fn parse(yaml: &str) -> Result<Vec<Automation>> {
    if yaml.len() > 131072 {
        return Err("YAML exceeds 128 KiB".into());
    }
    let mut p = Parser::new(yaml.chars());
    if !matches!(take(&mut p)?.0, YEvent::StreamStart)
        || !matches!(take(&mut p)?.0, YEvent::DocumentStart)
    {
        return Err("expected YAML document".into());
    }
    let root = node(&mut p, 0, &mut 0)?;
    if !matches!(take(&mut p)?.0, YEvent::DocumentEnd)
        || !matches!(take(&mut p)?.0, YEvent::StreamEnd)
    {
        return Err("exactly one YAML document required".into());
    }
    let mut rules = vec![];
    let mut ids = std::collections::BTreeSet::new();
    for n in root.seq()? {
        if rules.len() >= 32 {
            return Err("more than 32 automations".into());
        }
        let id = n.str("id")?;
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(n.err("duplicate/empty automation id"));
        }
        let mut unknown = vec![];
        if n.extra(&[
            "id",
            "alias",
            "mode",
            "max",
            "triggers",
            "conditions",
            "actions",
        ])? {
            unknown.push("unsupported automation field".into());
        }
        let mode = match n.opt("mode").map(Node::text).transpose()?.as_deref() {
            None | Some("single") => Mode::Single,
            Some("restart") => Mode::Restart,
            Some("queued") => Mode::Queued,
            Some("parallel") => Mode::Parallel,
            _ => return Err(n.err("unknown mode")),
        };
        let max = n
            .opt("max")
            .map(|v| v.text()?.parse::<usize>().map_err(|_| v.err("invalid max")))
            .transpose()?
            .unwrap_or(10);
        if max == 0 || max > 16 {
            return Err(n.err("max must be 1..16"));
        }
        rules.push(Automation {
            id,
            mode,
            max,
            source: n.source.clone(),
            unknown,
            triggers: n
                .get("triggers")?
                .seq()?
                .iter()
                .map(trigger)
                .collect::<Result<_>>()?,
            conditions: predicates(n.opt("conditions"))?,
            actions: actions(n.get("actions")?)?,
        });
    }
    Ok(rules)
}
#[cfg(test)]
mod tests {
    use super::*;
    const SIMPLE: &str = "- id: lamp\n  triggers:\n    - trigger: state\n      entity_id: binary_sensor.motion\n      to: 'on'\n  actions:\n    - action: light.turn_on\n      target:\n        entity_id: light.hall\n";
    #[test]
    fn parses_source_markers() {
        let r = parse(SIMPLE).unwrap();
        assert_eq!(r[0].actions[0].source.line, 7);
        assert_eq!(r[0].triggers[0].to.as_deref(), Some("on"));
    }
    #[test]
    fn rejects_duplicate_keys() {
        assert!(parse(&SIMPLE.replace("id: lamp", "id: lamp\n  id: other")).is_err());
    }
    #[test]
    fn rejects_aliases_and_tags() {
        for s in ["- &a {id: x}\n- *a", "!!str hi", "---\n[]\n---\n[]"] {
            assert!(parse(s).is_err());
        }
    }
    #[test]
    fn unsupported_is_unknown() {
        let r = parse(&SIMPLE.replace("light.turn_on", "lock.unlock")).unwrap();
        assert!(matches!(r[0].actions[0].effect, Effect::Unknown { .. }));
    }
    #[test]
    fn unsupported_trigger_duration_is_unknown() {
        let r = parse(&SIMPLE.replace("to: 'on'", "to: 'on'\n      for: 10")).unwrap();
        assert!(r[0].triggers[0].unknown.is_some());
    }
    #[test]
    fn duplicate_ids_and_depth_rejected() {
        assert!(parse(&format!("{SIMPLE}{SIMPLE}")).is_err());
        assert!(parse(&format!("{}0{}", "[".repeat(40), "]".repeat(40))).is_err());
    }
}
