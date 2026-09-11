# Command line

Build with `cargo build --release -p home-rule-cli`. Run from the repository root:

```sh
target/release/homecheck simulate --rules examples/alarm.yaml --scenario examples/alarm.json
target/release/homecheck verify --rules examples/alarm.yaml --scenario examples/alarm.json
target/release/homecheck minimize --rules examples/alarm.yaml --scenario examples/alarm.json
target/release/homecheck report --rules examples/alarm.yaml --scenario examples/alarm.json --format markdown --out review.md
target/release/homecheck diff --before examples/alarm.yaml --rules revised.yaml --scenario examples/alarm.json
```

`lint` reports static/potential hazards. `simulate` replays `events` and evaluates
invariants. `verify` replaces `events` with generated sequences from `alphabet`
and `gaps`, at depths zero through `search_depth`. Include unavailable/unknown
values and restart/reload events in that alphabet to explore those possibilities.
It does not explore values absent from your declared alphabet.

`minimize` removes external events while preserving the first confirmed violated
invariant. Its output wraps the reduced scenario under `scenario`; it is not a
minimum elapsed-time proof. `diff` searches each invariant separately with identical
bounds and reports uncertain comparisons honestly. `report` combines all evidence.

All commands accept `--format json|markdown|junit` and optional `--out NEW_FILE`.
Existing output files are never overwritten. No command alters input files.

Exit codes: 0 completed analysis without a confirmed violation, 1 confirmed
counterexample, 2 invalid input/IO error, 3 unknown/incomplete analysis. Static
potential hazards do not set exit 1. JUnit uses a failure for a counterexample
and a skipped result for an inconclusive check.
