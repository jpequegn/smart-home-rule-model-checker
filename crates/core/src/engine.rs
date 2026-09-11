use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trace {
    pub index: usize,
    pub at: u64,
    pub kind: String,
    pub automation: Option<String>,
    pub source: Option<Source>,
    pub entity: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub detail: String,
    pub state: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Simulation {
    pub trace: Vec<Trace>,
    pub final_state: BTreeMap<String, String>,
    pub unknown: Vec<String>,
    pub incomplete: Vec<String>,
    pub processed_steps: usize,
    pub horizon: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    Yes,
    No,
    Unknown,
}
pub fn evaluate(p: &Predicate, state: &BTreeMap<String, String>) -> Truth {
    match p {
        Predicate::Unknown { .. } => Truth::Unknown,
        Predicate::State { entity, value } => match state.get(entity) {
            Some(v) if v == value => Truth::Yes,
            Some(v) if v == "unknown" || v == "unavailable" => Truth::Unknown,
            Some(_) => Truth::No,
            None => Truth::Unknown,
        },
    }
}
pub fn evaluate_all(ps: &[Predicate], s: &BTreeMap<String, String>) -> Truth {
    let mut result = Truth::Yes;
    for p in ps {
        match evaluate(p, s) {
            Truth::No => return Truth::No,
            Truth::Unknown => result = Truth::Unknown,
            _ => {}
        }
    }
    result
}
fn matches(t: &Trigger, entity: &str, from: &str, to: &str) -> bool {
    t.unknown.is_none()
        && t.entity == entity
        && from != to
        && t.from.as_ref().is_none_or(|v| v == from)
        && t.to.as_ref().is_none_or(|v| v == to)
}
#[derive(Clone)]
struct Run {
    rule: usize,
    actions: VecDeque<Action>,
    waiting: Option<(Trigger, u64, bool, Source)>,
    queued: bool,
}
enum Work {
    External(Input),
    Changed(String, String, String),
    Resume(u64),
    Timeout(u64),
    Stale(String, u64),
}
struct Engine<'a> {
    s: &'a Scenario,
    rules: &'a [Automation],
    state: BTreeMap<String, String>,
    queue: BTreeMap<(u64, u64), Work>,
    serial: u64,
    now: u64,
    runs: BTreeMap<u64, Run>,
    next_run: u64,
    versions: BTreeMap<String, u64>,
    trace: Vec<Trace>,
    unknown: Vec<String>,
    incomplete: Vec<String>,
    steps: usize,
}
impl Engine<'_> {
    fn add(&mut self, at: u64, work: Work) {
        self.serial += 1;
        self.queue.insert((at, self.serial), work);
    }
    fn record(
        &mut self,
        kind: &str,
        rule: Option<usize>,
        source: Option<Source>,
        change: Option<(String, String, String)>,
        detail: String,
    ) {
        let (entity, before, after) = match change {
            Some((e, b, a)) => (Some(e), Some(b), Some(a)),
            None => (None, None, None),
        };
        self.trace.push(Trace {
            index: self.trace.len(),
            at: self.now,
            kind: kind.into(),
            automation: rule.map(|i| self.rules[i].id.clone()),
            source,
            entity,
            before,
            after,
            detail,
            state: self.state.clone(),
        });
    }
    fn uncertain(&mut self, reason: String) {
        if !self.unknown.contains(&reason) {
            self.unknown.push(reason);
        }
    }
    fn update(
        &mut self,
        entity: String,
        value: String,
        rule: Option<usize>,
        source: Option<Source>,
        kind: &str,
    ) {
        let before = self.state[&entity].clone();
        self.state.insert(entity.clone(), value.clone());
        let version = self.versions.entry(entity.clone()).or_default();
        *version += 1;
        let v = *version;
        if let Some(ttl) = self.s.entities[&entity].stale_after {
            self.add(self.now + ttl + 1, Work::Stale(entity.clone(), v));
        }
        self.record(
            kind,
            rule,
            source,
            Some((entity.clone(), before.clone(), value.clone())),
            String::new(),
        );
        if before != value {
            self.add(self.now, Work::Changed(entity, before, value));
        }
    }
    fn finish(&mut self, id: u64) {
        if let Some(run) = self.runs.remove(&id) {
            self.record("complete", Some(run.rule), None, None, format!("run {id}"));
            let next = self
                .runs
                .iter()
                .find(|(_, r)| r.rule == run.rule && r.queued)
                .map(|(id, _)| *id);
            if let Some(next) = next {
                self.runs.get_mut(&next).unwrap().queued = false;
                self.add(self.now, Work::Resume(next));
            }
        }
    }
    fn trigger(&mut self, entity: String, from: String, to: String) {
        let waiters: Vec<_> = self
            .runs
            .iter()
            .filter_map(|(id, r)| {
                r.waiting
                    .as_ref()
                    .filter(|(t, _, _, _)| matches(t, &entity, &from, &to))
                    .map(|_| *id)
            })
            .collect();
        for id in waiters {
            self.runs.get_mut(&id).unwrap().waiting = None;
            self.add(self.now, Work::Resume(id));
        }
        for i in 0..self.rules.len() {
            let rule = &self.rules[i];
            if !rule
                .triggers
                .iter()
                .any(|t| matches(t, &entity, &from, &to))
            {
                continue;
            }
            let verdict = evaluate_all(&rule.conditions, &self.state);
            self.record(
                "trigger",
                Some(i),
                Some(rule.source.clone()),
                None,
                format!("conditions {verdict:?}"),
            );
            if verdict == Truth::Unknown {
                self.uncertain(format!("{}: unknown trigger condition", rule.id));
            }
            if verdict != Truth::Yes || !rule.unknown.is_empty() {
                continue;
            }
            let ids: Vec<_> = self
                .runs
                .iter()
                .filter(|(_, r)| r.rule == i)
                .map(|(id, _)| *id)
                .collect();
            match rule.mode {
                Mode::Single if !ids.is_empty() => {
                    self.record(
                        "ignored",
                        Some(i),
                        None,
                        None,
                        "single mode already running".into(),
                    );
                    continue;
                }
                Mode::Restart => {
                    for id in &ids {
                        self.runs.remove(id);
                    }
                    if !ids.is_empty() {
                        self.record("cancel", Some(i), None, None, "restart mode".into());
                    }
                }
                Mode::Queued | Mode::Parallel if ids.len() >= rule.max => {
                    self.record("ignored", Some(i), None, None, "run max reached".into());
                    continue;
                }
                _ => {}
            }
            let queued = matches!(rule.mode, Mode::Queued) && !ids.is_empty();
            self.next_run += 1;
            let id = self.next_run;
            self.runs.insert(
                id,
                Run {
                    rule: i,
                    actions: rule.actions.clone().into(),
                    waiting: None,
                    queued,
                },
            );
            self.record(
                if queued { "queued" } else { "start" },
                Some(i),
                None,
                None,
                format!("run {id}"),
            );
            if !queued {
                self.add(self.now, Work::Resume(id));
            }
        }
    }
    fn resume(&mut self, id: u64) {
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        if run.queued || run.waiting.is_some() {
            return;
        }
        let Some(action) = run.actions.pop_front() else {
            self.finish(id);
            return;
        };
        let rule = run.rule;
        let source = Some(action.source.clone());
        let mut next = self.now;
        match action.effect {
            Effect::Set { entity, value } => {
                self.update(entity, value, Some(rule), source, "action")
            }
            Effect::Toggle { entity } => {
                let old = &self.state[&entity];
                if old == "unknown" || old == "unavailable" {
                    self.uncertain(format!("{}: toggle of unknown state", self.rules[rule].id));
                } else {
                    let value = if old == "on" { "off" } else { "on" };
                    self.update(entity, value.into(), Some(rule), source, "action");
                }
            }
            Effect::Delay { seconds } => {
                next += seconds;
                self.record(
                    "delay",
                    Some(rule),
                    source,
                    None,
                    format!("{seconds} seconds"),
                );
            }
            Effect::Condition { predicate } => {
                let v = evaluate(&predicate, &self.state);
                self.record("condition", Some(rule), source, None, format!("{v:?}"));
                if v == Truth::Unknown {
                    self.uncertain(format!("{}: unknown action condition", self.rules[rule].id));
                }
                if v != Truth::Yes {
                    self.finish(id);
                    return;
                }
            }
            Effect::Choose { branches, default } => {
                let mut selected = default;
                for b in branches {
                    match evaluate_all(&b.conditions, &self.state) {
                        Truth::Yes => {
                            selected = b.sequence;
                            break;
                        }
                        Truth::Unknown => {
                            self.uncertain(format!(
                                "{}: unknown choose condition",
                                self.rules[rule].id
                            ));
                            self.finish(id);
                            return;
                        }
                        _ => {}
                    }
                }
                let run = self.runs.get_mut(&id).unwrap();
                for a in selected.into_iter().rev() {
                    run.actions.push_front(a);
                }
                self.record("choose", Some(rule), source, None, "selected branch".into());
            }
            Effect::Wait {
                trigger,
                timeout,
                continue_on_timeout,
            } => {
                if trigger.unknown.is_some() {
                    self.uncertain("unsupported wait trigger".into());
                    self.finish(id);
                    return;
                }
                self.runs.get_mut(&id).unwrap().waiting = Some((
                    trigger,
                    self.now + timeout,
                    continue_on_timeout,
                    action.source,
                ));
                self.add(self.now + timeout, Work::Timeout(id));
                self.record(
                    "wait",
                    Some(rule),
                    source,
                    None,
                    format!("timeout {timeout}"),
                );
                return;
            }
            Effect::Unknown { reason } => {
                self.uncertain(reason.clone());
                self.record("unknown", Some(rule), source, None, reason);
                self.finish(id);
                return;
            }
        }
        self.add(next, Work::Resume(id));
    }
}
pub fn unsupported(rules: &[Automation]) -> Vec<String> {
    fn acts(actions: &[Action], out: &mut Vec<String>) {
        for a in actions {
            match &a.effect {
                Effect::Unknown { reason } => {
                    out.push(format!("{}:{}: {reason}", a.source.line, a.source.column))
                }
                Effect::Condition {
                    predicate: Predicate::Unknown { reason },
                } => out.push(reason.clone()),
                Effect::Choose { branches, default } => {
                    for b in branches {
                        for p in &b.conditions {
                            if let Predicate::Unknown { reason } = p {
                                out.push(reason.clone());
                            }
                        }
                        acts(&b.sequence, out);
                    }
                    acts(default, out);
                }
                Effect::Wait { trigger, .. } => {
                    if let Some(r) = &trigger.unknown {
                        out.push(r.clone());
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = vec![];
    for r in rules {
        out.extend(r.unknown.iter().map(|s| format!("{}: {s}", r.id)));
        for t in &r.triggers {
            if let Some(s) = &t.unknown {
                out.push(format!("{}: {s}", r.id));
            }
        }
        for p in &r.conditions {
            if let Predicate::Unknown { reason } = p {
                out.push(format!("{}: {reason}", r.id));
            }
        }
        acts(&r.actions, &mut out);
    }
    out.sort();
    out.dedup();
    out
}
pub fn simulate(s: &Scenario, rules: &[Automation]) -> Result<Simulation> {
    validate(s, rules)?;
    let mut e = Engine {
        s,
        rules,
        state: s
            .entities
            .iter()
            .map(|(k, v)| (k.clone(), v.initial.clone()))
            .collect(),
        queue: BTreeMap::new(),
        serial: 0,
        now: 0,
        runs: BTreeMap::new(),
        next_run: 0,
        versions: BTreeMap::new(),
        trace: vec![],
        unknown: unsupported(rules),
        incomplete: vec![],
        steps: 0,
    };
    e.record("initial", None, None, None, String::new());
    for event in &s.events {
        e.add(event.at, Work::External(event.input.clone()));
    }
    for (entity, v) in &s.entities {
        if let Some(ttl) = v.stale_after {
            e.add(ttl + 1, Work::Stale(entity.clone(), 0));
        }
    }
    while let Some(((at, _), _)) = e.queue.first_key_value() {
        if e.trace.len() >= 10000 || e.queue.len() > 20000 {
            e.incomplete.push("trace or queue budget exhausted".into());
            break;
        }
        if *at > s.limits.horizon {
            break;
        }
        if e.steps >= s.limits.max_steps {
            e.incomplete.push("step budget exhausted".into());
            break;
        }
        let ((at, _), work) = e.queue.pop_first().unwrap();
        e.now = at;
        e.steps += 1;
        match work {
            Work::External(Input::Set { entity, value }) => {
                e.update(entity, value, None, None, "external")
            }
            Work::External(Input::Restart | Input::Reload) => {
                e.runs.clear();
                e.record(
                    "restart",
                    None,
                    None,
                    None,
                    "cancel all runs; preserve entity state".into(),
                );
            }
            Work::Changed(entity, from, to) => e.trigger(entity, from, to),
            Work::Resume(id) => e.resume(id),
            Work::Timeout(id) => {
                let pending = e
                    .runs
                    .get(&id)
                    .and_then(|r| r.waiting.clone().map(|w| (r.rule, w)));
                if let Some((rule, (_, deadline, cont, source))) = pending
                    && deadline == e.now
                {
                    e.record(
                        "timeout",
                        Some(rule),
                        Some(source),
                        None,
                        format!("continue {cont}"),
                    );
                    if cont {
                        e.runs.get_mut(&id).unwrap().waiting = None;
                        e.add(e.now, Work::Resume(id));
                    } else {
                        e.finish(id);
                    }
                }
            }
            Work::Stale(entity, version) => {
                if *e.versions.get(&entity).unwrap_or(&0) == version
                    && e.state[&entity] != "unknown"
                {
                    let before = e.state.insert(entity.clone(), "unknown".into()).unwrap();
                    e.record(
                        "stale",
                        None,
                        None,
                        Some((entity.clone(), before.clone(), "unknown".into())),
                        "modeled freshness expired".into(),
                    );
                    e.add(e.now, Work::Changed(entity, before, "unknown".into()));
                }
            }
        }
    }
    if !e.runs.is_empty() {
        e.incomplete
            .push("active or queued runs beyond observation horizon".into());
    }
    if e.incomplete.is_empty() {
        e.now = s.limits.horizon;
        e.record("horizon", None, None, None, "observation boundary".into());
    }
    Ok(Simulation {
        trace: e.trace,
        final_state: e.state,
        unknown: e.unknown,
        incomplete: e.incomplete,
        processed_steps: e.steps,
        horizon: s.limits.horizon,
    })
}
