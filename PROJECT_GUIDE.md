# Using Homecheck

## What it can do

Homecheck models finite entities and automations, then explains how an external
event sequence produces state changes. Each modeled action links to its original
YAML line and column. Static warnings identify possible conflicts; invariant
violations identify a concrete failure in the modeled execution.

The browser and CLI use the same parser and engine. Neither connects to Home
Assistant. Lock, alarm, garage and arbitrary service integrations are not executed
or silently approximated. You can label a synthetic entity with a higher safety
tier to exercise stricter review requirements without controlling a real device.

## First experiment

1. Run the UI using the README commands. The default scenario turns motion on,
   turns it off, then enables alarm mode while a delayed light-off is pending.
2. Read the violation at four simulated seconds. Use its trace row's line button
   to inspect the delayed action.
3. Move the replay slider and watch the entity states. The canvas shows states
   over simulated time; the trace table preserves ordering within one second.
4. Choose **Guarded light-off**. This version checks alarm state again after the
   delay. The declared bounded search finds no counterexample.
5. Choose **Light toggle loop** or **Unsupported service**. The former demonstrates
   a concrete transition violation plus an execution budget limit. The latter
   remains unknown.
6. Return to the alarm example and run **Minimize**. Load the counterexample to
   replay the reduced scenario.
7. Export the project to preserve YAML and scenario together; export analysis
   evidence separately. Import restores a local project, then reruns analysis.

Edits invalidate old results. Analysis runs in a worker and can be cancelled.
There is no automatic localStorage persistence, telemetry or server-side upload.

## Defining a model

A project consists of automation YAML and JSON with:

- `entities`: finite state domains, initial values, safety tiers and optional
  synthetic `stale_after` thresholds.
- `events`: ordered timestamped external state changes or restart/reload events.
- `invariants`: never-state, pre-action guard, forbidden transition or response
  obligations with a deadline.
- `alphabet` and `gaps`: possible external events and time increments for search.
- `limits`: observation horizon, per-run work budget, event depth and case budget.

Use quoted YAML state values. Start from the examples. Every entity must be
declared. Add unavailable/unknown sensor events, restart/reload events and timing
choices explicitly if those possibilities matter. A search cannot invent events
outside your alphabet.

A response obligation begins when its `when` predicate becomes true and is
satisfied when `then` becomes true. While an obligation is pending, repeated
activations coalesce into the earliest outstanding deadline. The trace reports
the observation at which a missed deadline was detected. A deadline beyond the
observation horizon makes the result incomplete.

## Review habits

Keep the YAML and scenario under version control together. Write the invariant
before adjusting the automation. After an edit, use `diff` with the same scenario
to compare counterexample outcomes for each invariant separately. A newly found
counterexample is evidence within those bounds, not proof about every deployment.

Keep confirmed incidents as regression fixtures. Use a synthetic model first,
then manually compare it with actual Home Assistant traces and documentation.
Actual integration behavior, asynchronous service completion and nondeterministic
internal scheduling remain outside this model.

## Limits and costs

No API costs, GPU or model training. Search grows exponentially with alphabet size
and depth. Hard caps include 32 entities/rules/invariants, 512 actions, depth 5,
2,000 search cases, 10,000 work steps per replay and a 200,000-work-step aggregate
search budget. Output shows incomplete exploration instead of a clean result.
See the supported-subset contract for input and trace caps.

Static conflicts can be mutually exclusive at runtime. Potential warnings should
not be counted as true-positive failures. Unknown findings are not false positives
or passes: they identify an unmodeled part of the input.

## Useful extensions

- Import real Home Assistant traces offline and compare modeled versus observed
  state transitions. This would test simulator fidelity, which synthetic tests do
  not establish.
- Generate sensor-outage and timer-boundary scenarios from confirmed incidents.
- Add a constrained patch generator that proposes a post-delay guard, then checks
  the revised rule against the same counterexample. Keep approval manual.
- Export signed verification receipts for your personal evidence-verification
  projects, including source digests and every exploration bound.
- Use the model as a small benchmark for coding agents: give each agent a broken
  rule, withhold regression cases, and score repairs without giving it device access.
- Reuse the explicit queue and invariants to teach concurrency, model checking and
  the difference between testing one run and exploring a finite state space.
