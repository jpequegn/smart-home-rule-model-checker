# Implementation plan

Source: https://github.com/jpequegn/project-ideas/issues/225

The captured backlog is #1 through #8 in this repository:
1. Workspace, CI and compatibility contract.
2. Source-marked YAML parser and validated domain contracts.
3. Deterministic simulation and four run modes.
4. Static interaction analysis.
5. Bounded search, invariants and counterexample minimization.
6. CLI and evidence reports.
7. WASM browser workbench.
8. Corpus, properties, fuzzing, usage documentation and final verification.

Each task gets a branch, tests and a merged PR before the next task.

## Engineering decisions

Use Rust 1.92, serde contracts, yaml-rust2's marked event parser, clap and
wasm-bindgen. Keep typed formats inside the core crate rather than a separate
crate with no independent behavior. Use explicit discrete-state simulation as
requested by the source specification. The parser is not hand-written YAML.

The browser imports the same parser and analysis engine as the CLI. No provider
keys, LLM calls, native capture permissions or household integrations are needed.

## Evidence required for completion

Twenty-five named synthetic cases including benign controls; all four modes;
restart and availability cases; property tests; bounded fuzz smoke; CLI report
checks; native/WASM parity; desktop/mobile browser checks. Unknown constructs,
resource exhaustion and pending activity must never become an unqualified pass.
