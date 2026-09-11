# Compatibility and safety contract

This is an offline model, not a Home Assistant runtime emulator or safety certificate.
No device API, credentials, discovery, execution or deployment is provided.

Input consists of Home Assistant-style automation YAML and a separate JSON model
of finite entity domains, initial state, explicit external events, invariants and
search budgets. Examples use synthetic entities only.

Planned supported subset: state triggers with literal entity/from/to; state
conditions; literal light/input_boolean/switch turn_on, turn_off and toggle intents;
integer-second delays; ordered actions; choose branches; bounded state-trigger
waits; single, restart, queued and parallel modes. State trigger durations, Jinja,
attributes, dynamic targets, device integrations and arbitrary service behavior
are unknown unless explicitly modeled. Unknown syntax is not dropped silently.
Aliases, tags, duplicate keys and malformed structures are rejected.

Time is integer simulated seconds. External events at the same instant retain
input order. Automation iteration follows source order. Search enumerates bounded
external event sequences; it does not prove every possible internal scheduler
interleaving. A restart/reload cancels active and queued runs and timers; entity
values remain in the declared model. This is an explicit modeling assumption.

A successful bounded search means no counterexample in the recorded finite event
alphabet, timing choices and depth. It is never an unbounded proof. Budget
exhaustion, unresolved constructs and insufficient observation time are reported
as incomplete or unknown.

Source positions are one-based YAML line/column start markers, not guessed text
matches. Lint findings are potential hazards, distinct from simulated violations.

References checked 2026-09-11:
- https://www.home-assistant.io/docs/automation/modes/
- https://www.home-assistant.io/docs/scripts/
