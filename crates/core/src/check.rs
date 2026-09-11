use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Violation {
    pub invariant: String,
    pub at: u64,
    pub trace_index: usize,
    pub source: Option<Source>,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub status: String,
    pub violations: Vec<Violation>,
    pub unknown: Vec<String>,
    pub incomplete: Vec<String>,
    pub simulation: Simulation,
}
pub fn check(s: &Scenario, rules: &[Automation]) -> Result<Check> {
    let sim = simulate(s, rules)?;
    let mut violations = vec![];
    let mut unknown = sim.unknown.clone();
    let mut incomplete = sim.incomplete.clone();
    for invariant in &s.invariants {
        let mut pending: Option<(u64, usize)> = None;
        let mut was_when = false;
        for (idx, t) in sim.trace.iter().enumerate() {
            let before = if idx == 0 {
                &t.state
            } else {
                &sim.trace[idx - 1].state
            };
            let mut failure = None;
            match invariant {
                Invariant::Never { conditions, .. } => match evaluate_all(conditions, &t.state) {
                    Truth::Yes => failure = Some("prohibited state reached".to_string()),
                    Truth::Unknown => {
                        unknown.push(format!("{}: unknown state predicate", invariant.id()))
                    }
                    _ => {}
                },
                Invariant::Guard {
                    entity,
                    value,
                    requires,
                    ..
                } if t.kind == "action"
                    && t.entity.as_ref() == Some(entity)
                    && t.after.as_ref() == Some(value) =>
                {
                    match evaluate_all(requires, before) {
                        Truth::No => failure = Some("action guard violated".into()),
                        Truth::Unknown => {
                            unknown.push(format!("{}: unknown action guard", invariant.id()))
                        }
                        _ => {}
                    }
                }
                Invariant::Transition {
                    entity, from, to, ..
                } if t.entity.as_ref() == Some(entity)
                    && t.before.as_ref() == Some(from)
                    && t.after.as_ref() == Some(to) =>
                {
                    failure = Some("forbidden transition".into())
                }
                Invariant::Response {
                    when, then, within, ..
                } => {
                    if let Some((deadline, _)) = pending
                        && deadline < t.at
                    {
                        if evaluate(then, before) == Truth::No {
                            failure = Some(format!("response deadline {deadline} missed"));
                        } else if evaluate(then, before) == Truth::Unknown {
                            unknown
                                .push(format!("{}: unknown response at deadline", invariant.id()));
                        }
                        pending = None;
                    }
                    let w = evaluate(when, &t.state);
                    let goal = evaluate(then, &t.state);
                    if w == Truth::Unknown || goal == Truth::Unknown {
                        unknown.push(format!("{}: unknown temporal predicate", invariant.id()));
                    }
                    if w == Truth::Yes && !was_when && pending.is_none() {
                        pending = Some((t.at + within, idx));
                    }
                    if goal == Truth::Yes {
                        pending = None;
                    }
                    was_when = w == Truth::Yes;
                    if t.kind == "horizon"
                        && let Some((deadline, _)) = pending
                    {
                        if deadline <= t.at && goal == Truth::No {
                            failure = Some(format!("response deadline {deadline} missed"));
                        } else if deadline > t.at {
                            incomplete.push(format!(
                                "{}: response deadline beyond horizon",
                                invariant.id()
                            ));
                        }
                    }
                }
                _ => {}
            }
            if let Some(message) = failure {
                violations.push(Violation {
                    invariant: invariant.id().into(),
                    at: t.at,
                    trace_index: idx,
                    source: t.source.clone(),
                    message,
                });
                break;
            }
        }
    }
    unknown.sort();
    unknown.dedup();
    incomplete.sort();
    incomplete.dedup();
    let status = if !unknown.is_empty() {
        "unknown"
    } else if !violations.is_empty() {
        "violation"
    } else if !incomplete.is_empty() {
        "incomplete"
    } else {
        "no_violation_in_trace"
    };
    Ok(Check {
        status: status.into(),
        violations,
        unknown,
        incomplete,
        simulation: sim,
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verification {
    pub status: String,
    pub cases: usize,
    pub total_steps: usize,
    pub depth: usize,
    pub horizon: u64,
    pub alphabet: Vec<Input>,
    pub gaps: Vec<u64>,
    pub max_cases: usize,
    pub max_steps_per_case: usize,
    pub witness: Option<Scenario>,
    pub check: Option<Check>,
    pub reasons: Vec<String>,
    pub shortest_by_event_count: bool,
}
pub fn verify(s: &Scenario, rules: &[Automation]) -> Result<Verification> {
    validate(s, rules)?;
    if s.invariants.is_empty() {
        return Err("verification requires at least one invariant".into());
    }
    let mut out = Verification {
        status: "no_counterexample_within_bounds".into(),
        cases: 0,
        total_steps: 0,
        depth: s.limits.search_depth,
        horizon: s.limits.horizon,
        alphabet: s.alphabet.clone(),
        gaps: s.gaps.clone(),
        max_cases: s.limits.max_cases,
        max_steps_per_case: s.limits.max_steps,
        witness: None,
        check: None,
        reasons: unsupported(rules),
        shortest_by_event_count: false,
    };
    fn visit(
        s: &Scenario,
        rules: &[Automation],
        events: &mut Vec<Event>,
        remaining: usize,
        out: &mut Verification,
    ) -> Result<bool> {
        if remaining == 0 {
            if out.cases >= s.limits.max_cases || out.total_steps >= 200000 {
                out.reasons.push("search budget exhausted".into());
                return Ok(true);
            }
            let mut candidate = s.clone();
            candidate.events = events.clone();
            let c = check(&candidate, rules)?;
            out.cases += 1;
            out.total_steps += c.simulation.processed_steps;
            if c.status == "violation" {
                out.shortest_by_event_count = out.reasons.is_empty();
                out.status = "counterexample".into();
                out.witness = Some(candidate);
                out.check = Some(c);
                return Ok(true);
            }
            out.reasons.extend(c.unknown);
            out.reasons.extend(c.incomplete);
            out.reasons.sort();
            out.reasons.dedup();
            return Ok(false);
        }
        for gap in &s.gaps {
            let at = events.last().map_or(0, |e| e.at) + gap;
            if at > s.limits.horizon {
                continue;
            }
            for input in &s.alphabet {
                events.push(Event {
                    at,
                    input: input.clone(),
                });
                if visit(s, rules, events, remaining - 1, out)? {
                    return Ok(true);
                }
                events.pop();
            }
        }
        Ok(false)
    }
    for depth in 0..=s.limits.search_depth {
        if visit(s, rules, &mut vec![], depth, &mut out)? {
            break;
        }
    }
    out.reasons.sort();
    out.reasons.dedup();
    if out.witness.is_none() && !out.reasons.is_empty() {
        out.status = "inconclusive".into();
    }
    Ok(out)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Minimized {
    pub scenario: Scenario,
    pub invariant: String,
    pub original_events: usize,
    pub attempts: usize,
    pub deletion_minimal: bool,
    pub check: Check,
}
pub fn minimize(s: &Scenario, rules: &[Automation]) -> Result<Minimized> {
    let first = check(s, rules)?;
    if first.status != "violation" {
        return Err("a confirmed modeled violation is required".into());
    }
    let target = first.violations[0].invariant.clone();
    let mut reduced = s.clone();
    let mut attempts = 0;
    let mut idx = 0;
    while idx < reduced.events.len() && attempts < 200 {
        let mut candidate = reduced.clone();
        candidate.events.remove(idx);
        attempts += 1;
        let c = check(&candidate, rules)?;
        if c.status == "violation" && c.violations.iter().any(|v| v.invariant == target) {
            reduced = candidate;
            idx = 0;
        } else {
            idx += 1;
        }
    }
    let deletion_minimal = idx == reduced.events.len();
    let final_check = check(&reduced, rules)?;
    Ok(Minimized {
        scenario: reduced,
        invariant: target,
        original_events: s.events.len(),
        attempts,
        deletion_minimal,
        check: final_check,
    })
}
