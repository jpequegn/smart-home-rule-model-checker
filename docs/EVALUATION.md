# Evaluation record

## Synthetic corpus

`examples/corpus.json` contains 30 independent versioned packs. Each embeds
automation YAML, a complete scenario, a manually specified outcome, action
timestamps and expected lint codes. Cases cover the alarm-delay failure and repair,
all four run modes, run caps, restart/reload, successful/timed-out waits, action
conditions, choose/default, cycles, unknown services/templates/duration triggers,
broad targets, stale/unavailable sensors, condition mismatches, higher-tier guard
requirements, opposing and benign concurrent writes, and response deadlines.

The Rust corpus test checks every expected outcome, timestamp sequence and required
lint code. Native/WASM parity compares full replay and lint JSON for all 30 packs;
the five main operations also match on the alarm fixture. A wasm-bindgen-test runs
the golden outcomes inside WASM.

These are regression results on authored synthetic cases, not measured recall or
precision on real Home Assistant installations.

## Additional checks

- Focused parser, contract, concurrency, invariant, search, CLI and report tests.
- Property tests run 64 cases each for deterministic replay, preserved minimized
  violations, witness preservation when expanding the event alphabet, and malformed
  YAML handling.
- Targeted mutations cover safety condition changes, delay changes and invalid
  targets; mode tests distinguish single/restart/queued/parallel behavior.
- Parser fuzz target exercises YAML, strict JSON and validated simulation.
  A 5,000-iteration sanitizer-backed smoke run used nightly Rust, max length 8,192,
  timeout 5s and RSS cap 1,024 MiB. It completed without a crash or sanitizer report.
  This is a short smoke test, not proof of parser safety.
- Desktop 1440x1000 and mobile 390x844 browser screenshots inspected; canvas
  nonblank, no horizontal overflow. Safe/unknown outcomes, minimize/load and local
  import/export exercised. No remote resource requests observed. Test Chrome closed.

## Reproduce

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build -p home-rule-cli --locked
wasm-pack build crates/wasm --target web --out-dir ../../web/pkg --release
wasm-pack test --node crates/wasm
node scripts/parity.mjs
cd web
npm ci
npm run build
```

For a longer optional fuzz run, install cargo-fuzz and a nightly toolchain, then
run `cargo +nightly fuzz run parse_model -- -max_total_time=60 -max_len=8192`
from the repository root. Keep generated corpus/artifacts local.

## Unproven properties

No claim of complete Home Assistant equivalence, exhaustive internal scheduling,
unbounded safety, mutation-test coverage of all code or real-device validation.
Resource-limited search remains inconclusive. Source-issue completion refers to
the documented offline subset, not certification of household automations.
