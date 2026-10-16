# FIXLOG

Working log of the important fixes from each build session. This file is a
temporary record — it will be rewritten into the final documentation when
the next release is cut.

## Session 1 — 2026-10-16

### Docs audited
- `docs/*.md` + `README.md` checked against the code: accurate, with one
  wording note (a profile rename is a save-new + delete-old, not an atomic
  move — ARCHITECTURE.md says "moves the file"). Reword in the final doc
  rewrite.

### Fixed (critical): zombie RUNNING state after a worker panic
- Symptom: a panic inside the click loop (enigo internals, e.g. the libei
  portal path) killed the worker thread while the running flag stayed
  `true` — the engine showed green "Running" forever, no clicks, no error,
  and the panic hook in `main.rs` swallowed the message.
- Fix (`src/clicker/engine.rs`): the whole worker body (enigo creation +
  click loop) is now wrapped in `catch_unwind`; a crash records
  `Engine crashed: <message>` in the engine error slot and clears the
  running flag → the UI's per-tick drain surfaces the red ERROR state and
  the profile stays re-armable. Added `panic_message()` helper with unit
  tests.
- Verified: `cargo test --release` (3 tests pass) and `cargo build --release`.
