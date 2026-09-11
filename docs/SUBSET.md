# Compatibility and safety contract

This is an offline model, not a Home Assistant runtime emulator or safety certificate.
No device API, credentials, discovery, execution or deployment is provided.

Input consists of a top-level YAML sequence using plural triggers/conditions/actions
keys and a separate JSON model
of finite entity domains, initial state, explicit external events, invariants and
search budgets. Examples use synthetic entities only.

Supported subset: state triggers with literal entity/from/to; state
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

External events are enqueued before internal work. At equal timestamps the queue
uses insertion order: all predeclared external events run before newly generated
internal work. Trigger admission evaluates current state, not a historical trigger
snapshot. Reports do not claim exhaustive internal-scheduler interleavings.

Synthetic freshness expires at last update + stale_after + 1 second and changes the
modeled value to unknown. This is not an assertion about Home Assistant's sensor
expiry behavior. Reads comparing an unknown/unavailable value with an ordinary
value remain unknown; an explicit comparison with unknown/unavailable is defined.

Input caps: 128 KiB YAML, 256 KiB scenario JSON, YAML depth 24, 10,000 YAML nodes,
action depth 16, 512 actions and finite entity domains of at most 16 values each.
Run queues are capped, traces stop at approximately 10,000 entries, and search
stops after 200,000 processed work items across cases. A final work item can emit
several trace rows, so the trace limit is checked between work items.

State/transition invariants can refer to initial or external state with no YAML
source. Those findings link to the exact trace index instead. Action findings link
to YAML. Unsupported lint identifies the containing automation and includes more
specific action locations in its reason when available.

References checked 2026-09-11:
- https://www.home-assistant.io/docs/automation/modes/
- https://www.home-assistant.io/docs/scripts/
