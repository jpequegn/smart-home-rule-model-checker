use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub certainty: String,
    pub message: String,
    pub automations: Vec<String>,
    pub sources: Vec<Source>,
    pub tier: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interaction {
    pub from: String,
    pub to: String,
    pub entity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lint {
    pub findings: Vec<Finding>,
    pub interactions: Vec<Interaction>,
    pub components: Vec<Vec<String>>,
    pub reads: BTreeMap<String, Vec<String>>,
    pub writes: BTreeMap<String, Vec<String>>,
}
fn walk<'a>(actions: &'a [Action], out: &mut Vec<&'a Action>) {
    for a in actions {
        out.push(a);
        if let Effect::Choose { branches, default } = &a.effect {
            for b in branches {
                walk(&b.sequence, out);
            }
            walk(default, out);
        }
    }
}
fn write(a: &Action) -> Option<(&str, &str)> {
    match &a.effect {
        Effect::Set { entity, value } => Some((entity, value)),
        Effect::Toggle { entity } => Some((entity, "*")),
        _ => None,
    }
}
fn contradictory(ps: &[Predicate]) -> bool {
    let mut values = BTreeMap::new();
    for p in ps {
        if let Predicate::State { entity, value } = p
            && let Some(old) = values.insert(entity, value)
            && old != value
        {
            return true;
        }
    }
    false
}
pub fn lint(s: &Scenario, rules: &[Automation]) -> Result<Lint> {
    validate(s, rules)?;
    let mut out = Lint {
        findings: vec![],
        interactions: vec![],
        components: vec![],
        reads: BTreeMap::new(),
        writes: BTreeMap::new(),
    };
    let actions: Vec<Vec<&Action>> = rules
        .iter()
        .map(|r| {
            let mut v = vec![];
            walk(&r.actions, &mut v);
            v
        })
        .collect();
    let mut reach = vec![vec![false; rules.len()]; rules.len()];
    for (i, r) in rules.iter().enumerate() {
        let mut reads: BTreeSet<String> = r
            .triggers
            .iter()
            .filter(|t| t.unknown.is_none())
            .map(|t| t.entity.clone())
            .collect();
        let mut predicates: Vec<&Predicate> = r.conditions.iter().collect();
        for a in &actions[i] {
            match &a.effect {
                Effect::Condition { predicate } => predicates.push(predicate),
                Effect::Choose { branches, .. } => {
                    for b in branches {
                        predicates.extend(&b.conditions);
                    }
                }
                Effect::Wait { trigger, .. } => {
                    reads.insert(trigger.entity.clone());
                }
                _ => {}
            }
        }
        for p in predicates {
            if let Predicate::State { entity, .. } = p {
                reads.insert(entity.clone());
            }
        }
        let writes: BTreeSet<String> = actions[i]
            .iter()
            .filter_map(|a| write(a).map(|(e, _)| e.to_string()))
            .collect();
        out.reads
            .insert(r.id.clone(), reads.iter().cloned().collect());
        out.writes
            .insert(r.id.clone(), writes.iter().cloned().collect());
        let mut add = |code: &str, message: String, source: Source, certainty: &str| {
            let tier = writes
                .iter()
                .filter_map(|e| s.entities.get(e))
                .map(|e| match e.tier {
                    Tier::Observational => 0,
                    Tier::Comfort => 1,
                    Tier::Property => 2,
                    Tier::AccessSafety => 3,
                })
                .max()
                .unwrap_or(0);
            out.findings.push(Finding {
                code: code.into(),
                certainty: certainty.into(),
                message,
                automations: vec![r.id.clone()],
                sources: vec![source],
                tier: ["observational", "comfort", "property", "access_safety"][tier].into(),
            });
        };
        if contradictory(&r.conditions)
            || r.triggers.iter().all(|t| {
                t.unknown.is_none()
                    && t.to.as_ref().is_some_and(|to| {
                        r.conditions.iter().any(
                    |p| matches!(p,Predicate::State{entity,value} if entity==&t.entity&&value!=to),
                )
                    })
            })
        {
            add(
                "unreachable_conditions",
                "Trigger-time state constraints contradict every modeled trigger".into(),
                r.source.clone(),
                "static",
            );
        }
        if actions[i]
            .iter()
            .any(|a| matches!(a.effect, Effect::Delay { .. }))
            && !writes.is_empty()
        {
            add("delayed_write","Delayed writes may outlive trigger-time safety conditions; replay with changed state".into(),r.source.clone(),"potential");
        }
        for a in &actions[i] {
            if let Effect::Choose { branches, .. } = &a.effect {
                for b in branches {
                    if contradictory(&b.conditions) {
                        add(
                            "unreachable_branch",
                            "Choose branch has contradictory state predicates".into(),
                            a.source.clone(),
                            "static",
                        );
                    }
                }
            }
        }
        if writes.len() > 8 {
            add(
                "high_fanout",
                "More than eight literal entity targets".into(),
                r.source.clone(),
                "potential",
            );
        }
        for entity in &reads {
            if s.entities
                .get(entity)
                .is_some_and(|e| e.stale_after.is_some())
            {
                add(
                    "freshness_assumption",
                    format!("{entity} has modeled freshness expiry"),
                    r.source.clone(),
                    "potential",
                );
            }
        }
        for a in &actions[i] {
            if let Some((entity, value)) = write(a) {
                if matches!(s.entities[entity].tier,Tier::Property|Tier::AccessSafety) && !s.invariants.iter().any(|inv|matches!(inv,Invariant::Guard{entity:e,value:v,..} if e==entity&&(value=="*"||v==value))) {
                    add("missing_safety_guard",format!("{entity} write has no matching action-guard invariant"),a.source.clone(),"potential");
                }
                for (j, target) in rules.iter().enumerate() {
                    if target.triggers.iter().any(|t| {
                        t.unknown.is_none()
                            && t.entity == entity
                            && (value == "*" || t.to.as_ref().is_none_or(|v| v == value))
                    }) {
                        reach[i][j] = true;
                        out.interactions.push(Interaction {
                            from: r.id.clone(),
                            to: target.id.clone(),
                            entity: entity.into(),
                        });
                    }
                }
            }
        }
    }
    for i in 0..rules.len() {
        for j in i + 1..rules.len() {
            for a in &actions[i] {
                for b in &actions[j] {
                    if let (Some((ea, va)), Some((eb, vb))) = (write(a), write(b))
                        && ea == eb
                        && (va != vb || va == "*")
                    {
                        out.findings.push(Finding{code:"opposing_writes".into(),certainty:"potential".into(),message:format!("Opposing or toggling intents for {ea}; conditions may make them mutually exclusive"),automations:vec![rules[i].id.clone(),rules[j].id.clone()],sources:vec![a.source.clone(),b.source.clone()],tier:format!("{:?}",s.entities[ea].tier).to_lowercase()});
                    }
                }
            }
        }
    }
    for k in 0..rules.len() {
        for i in 0..rules.len() {
            for j in 0..rules.len() {
                reach[i][j] |= reach[i][k] && reach[k][j];
            }
        }
    }
    let mut assigned = BTreeSet::new();
    for (i, row) in reach.iter().enumerate() {
        if assigned.contains(&i) || !row[i] {
            continue;
        }
        let ids: Vec<_> = (0..rules.len())
            .filter(|j| reach[i][*j] && reach[*j][i])
            .collect();
        for j in &ids {
            assigned.insert(*j);
        }
        let component: Vec<_> = ids.iter().map(|j| rules[*j].id.clone()).collect();
        out.findings.push(Finding{code:"trigger_cycle".into(),certainty:"potential".into(),message:"Cyclic trigger/write interaction; simulate to distinguish stable states from repeated toggling".into(),automations:component.clone(),sources:ids.iter().map(|j|rules[*j].source.clone()).collect(),tier:"review".into()});
        out.components.push(component);
    }
    for reason in unsupported(rules) {
        out.findings.push(Finding {
            code: "unsupported".into(),
            certainty: "unknown".into(),
            message: reason,
            automations: vec![],
            sources: vec![],
            tier: "unknown".into(),
        });
    }
    Ok(out)
}
