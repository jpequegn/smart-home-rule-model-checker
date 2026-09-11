use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Observational,
    Comfort,
    Property,
    AccessSafety,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub domain: Vec<String>,
    pub initial: String,
    pub tier: Tier,
    pub stale_after: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    State { entity: String, value: String },
    Unknown { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    pub entity: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub source: Source,
    pub unknown: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Single,
    Restart,
    Queued,
    Parallel,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Branch {
    pub conditions: Vec<Predicate>,
    pub sequence: Vec<Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub source: Source,
    pub effect: Effect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    Set {
        entity: String,
        value: String,
    },
    Toggle {
        entity: String,
    },
    Delay {
        seconds: u64,
    },
    Condition {
        predicate: Predicate,
    },
    Choose {
        branches: Vec<Branch>,
        default: Vec<Action>,
    },
    Wait {
        trigger: Trigger,
        timeout: u64,
        continue_on_timeout: bool,
    },
    Unknown {
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Automation {
    pub id: String,
    pub mode: Mode,
    pub max: usize,
    pub triggers: Vec<Trigger>,
    pub conditions: Vec<Predicate>,
    pub actions: Vec<Action>,
    pub source: Source,
    pub unknown: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Set { entity: String, value: String },
    Restart,
    Reload,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub at: u64,
    pub input: Input,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Invariant {
    Never {
        id: String,
        conditions: Vec<Predicate>,
    },
    Guard {
        id: String,
        entity: String,
        value: String,
        requires: Vec<Predicate>,
    },
    Transition {
        id: String,
        entity: String,
        from: String,
        to: String,
    },
    Response {
        id: String,
        when: Predicate,
        then: Predicate,
        within: u64,
    },
}
impl Invariant {
    pub fn id(&self) -> &str {
        match self {
            Self::Never { id, .. }
            | Self::Guard { id, .. }
            | Self::Transition { id, .. }
            | Self::Response { id, .. } => id,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub horizon: u64,
    pub max_steps: usize,
    pub search_depth: usize,
    pub max_cases: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub version: u32,
    pub title: String,
    pub entities: BTreeMap<String, Entity>,
    pub events: Vec<Event>,
    pub invariants: Vec<Invariant>,
    pub alphabet: Vec<Input>,
    pub gaps: Vec<u64>,
    pub limits: Limits,
}
pub fn check_value(s: &Scenario, entity: &str, value: &str) -> Result<()> {
    let e = s
        .entities
        .get(entity)
        .ok_or_else(|| format!("undeclared entity: {entity}"))?;
    if value == "unknown" || value == "unavailable" || e.domain.iter().any(|v| v == value) {
        Ok(())
    } else {
        Err(format!("value {value} outside {entity} domain"))
    }
}
pub fn check_predicate(s: &Scenario, p: &Predicate) -> Result<()> {
    if let Predicate::State { entity, value } = p {
        check_value(s, entity, value)?;
    }
    Ok(())
}
fn check_trigger(s: &Scenario, t: &Trigger) -> Result<()> {
    if t.unknown.is_none() {
        if !s.entities.contains_key(&t.entity) {
            return Err(format!("undeclared trigger entity: {}", t.entity));
        }
        for v in [&t.from, &t.to].into_iter().flatten() {
            check_value(s, &t.entity, v)?;
        }
    }
    Ok(())
}
fn check_actions(s: &Scenario, actions: &[Action], depth: usize, count: &mut usize) -> Result<()> {
    if depth > 16 {
        return Err("action depth exceeds 16".into());
    }
    for a in actions {
        *count += 1;
        if *count > 512 {
            return Err("more than 512 actions".into());
        }
        match &a.effect {
            Effect::Set { entity, value } => check_value(s, entity, value)?,
            Effect::Toggle { entity } => {
                check_value(s, entity, "on")?;
                check_value(s, entity, "off")?;
            }
            Effect::Delay { seconds } if *seconds > 86400 => {
                return Err("delay exceeds 86400 seconds".into());
            }
            Effect::Wait {
                trigger, timeout, ..
            } => {
                check_trigger(s, trigger)?;
                if *timeout > 86400 {
                    return Err("wait timeout exceeds 86400".into());
                }
            }
            Effect::Condition { predicate } => check_predicate(s, predicate)?,
            Effect::Choose { branches, default } => {
                for b in branches {
                    for p in &b.conditions {
                        check_predicate(s, p)?;
                    }
                    check_actions(s, &b.sequence, depth + 1, count)?;
                }
                check_actions(s, default, depth + 1, count)?;
            }
            _ => {}
        }
    }
    Ok(())
}
pub fn validate(s: &Scenario, rules: &[Automation]) -> Result<()> {
    if s.version != 1 || s.title.len() > 200 {
        return Err("invalid version or title".into());
    }
    if s.entities.is_empty()
        || s.entities.len() > 32
        || rules.len() > 32
        || s.events.len() > 100
        || s.invariants.len() > 32
    {
        return Err("model size limit".into());
    }
    let l = &s.limits;
    if l.horizon > 86400
        || l.max_steps == 0
        || l.max_steps > 10000
        || l.search_depth > 5
        || l.max_cases == 0
        || l.max_cases > 2000
    {
        return Err("invalid analysis limits".into());
    }
    if s.alphabet.len() > 12
        || s.gaps.is_empty()
        || s.gaps.len() > 4
        || s.gaps.iter().any(|g| *g > 86400)
    {
        return Err("invalid alphabet or gaps".into());
    }
    for (id, e) in &s.entities {
        if id.is_empty()
            || id.len() > 100
            || e.domain.is_empty()
            || e.domain.len() > 16
            || e.domain.iter().any(|v| v.is_empty() || v.len() > 100)
            || e.domain.iter().collect::<BTreeSet<_>>().len() != e.domain.len()
        {
            return Err("invalid entity domain".into());
        }
        check_value(s, id, &e.initial)?;
    }
    for input in s.events.iter().map(|e| &e.input).chain(s.alphabet.iter()) {
        if let Input::Set { entity, value } = input {
            check_value(s, entity, value)?;
        }
    }
    if s.events.iter().any(|e| e.at > l.horizon) || s.events.windows(2).any(|w| w[0].at > w[1].at) {
        return Err("events must be ordered within horizon".into());
    }
    let mut ids = BTreeSet::new();
    for i in &s.invariants {
        if i.id().is_empty() || !ids.insert(i.id()) {
            return Err("duplicate/empty invariant id".into());
        }
        match i {
            Invariant::Never { conditions, .. }
            | Invariant::Guard {
                requires: conditions,
                ..
            } => {
                if conditions.is_empty() {
                    return Err("empty invariant predicate".into());
                }
                for p in conditions {
                    check_predicate(s, p)?;
                }
                if let Invariant::Guard { entity, value, .. } = i {
                    check_value(s, entity, value)?;
                }
            }
            Invariant::Transition {
                entity, from, to, ..
            } => {
                check_value(s, entity, from)?;
                check_value(s, entity, to)?;
            }
            Invariant::Response {
                when, then, within, ..
            } => {
                check_predicate(s, when)?;
                check_predicate(s, then)?;
                if *within > 86400 {
                    return Err("response deadline limit".into());
                }
            }
        }
    }
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for r in rules {
        if r.id.is_empty() || r.id.len() > 100 || !ids.insert(&r.id) {
            return Err("duplicate/empty automation id".into());
        }
        if r.max == 0
            || r.max > 16
            || r.triggers.is_empty()
            || r.triggers.len() > 16
            || r.conditions.len() > 32
        {
            return Err("invalid automation bounds".into());
        }
        for t in &r.triggers {
            check_trigger(s, t)?;
        }
        for p in &r.conditions {
            check_predicate(s, p)?;
        }
        check_actions(s, &r.actions, 0, &mut count)?;
    }
    Ok(())
}
