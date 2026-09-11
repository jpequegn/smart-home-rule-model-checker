# Homecheck

Offline verification of a documented Home Assistant automation subset, with a
Rust CLI and browser-local WebAssembly workbench.

Inspect conflicting rules, replay delayed actions, search bounded event sequences,
and reduce a failing scenario to a smaller counterexample. No device access,
credentials, cloud service, or LLM is required.

## Run

Rust 1.92, Node 22.12+ and npm are required. Install wasm-pack 0.13.1 once.

```sh
cargo install wasm-pack --version 0.13.1 --locked
cargo build --release -p home-rule-cli
target/release/homecheck simulate --rules examples/alarm.yaml --scenario examples/alarm.json
```

The example intentionally returns exit 1: a delayed light-off violates the alarm
guard. Exit 3 means inconclusive; neither result should be treated as safe.

For the UI:

```sh
cd web
npm ci
npm run wasm
npm run dev
```

Open the local URL printed by Vite. Choose the guarded example to compare results.
The UI uploads nothing. Close the tab and stop Vite when finished.

## Scope

- Literal state triggers and conditions; ordered light/switch/input_boolean
  on/off/toggle intents; integer delays, choose branches and bounded waits.
- Single, restart, queued and parallel modes under explicit scheduling assumptions.
- State, action-guard, transition and bounded-response invariants.
- Source-linked lint, replay, bounded search, minimization and revision comparison.
- Thirty versioned synthetic packs in `examples/corpus.json`.

This is not a Home Assistant runtime emulator or a household safety certificate.
Unsupported constructs remain unknown. "No counterexample within bounds" applies
only to the supplied event alphabet, timing gaps, horizon and depth.

[Usage and extensions](PROJECT_GUIDE.md) · [CLI](docs/CLI.md) ·
[Supported subset](docs/SUBSET.md) · [Evaluation](docs/EVALUATION.md) ·
[Implementation plan](docs/PLAN.md)

Source: [project-ideas #225](https://github.com/jpequegn/project-ideas/issues/225).
