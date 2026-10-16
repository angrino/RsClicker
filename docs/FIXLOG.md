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

## Session 2 — 2026-10-16

### Fixed (critical): UI freeze on stop/start (worker join on the UI thread)
- Symptom: `ClickEngine::stop()` joined the worker thread inside the iced
  update thread — every stop (trigger, disarm, restart, drop) froze the UI
  for the duration of an in-flight click, up to indefinitely when a
  libei/portal call hangs.
- Fix (`src/clicker/engine.rs`):
  - `stop()` never joins on the caller thread: it waits at most
    `STOP_WAIT_MS` (50 ms) on a per-worker "done" signal, then hands the
    join to a detached `rsclicker-join` reaper thread.
  - Worker tails now touch shared state conditionally: a per-engine
    generation counter is bumped by `start()` *before* raising the flag,
    and tails (flag clear, crash/NoBackend error reports) act only while
    they are the current generation — without the old blocking join, the
    tail of an in-flight old worker could otherwise clobber the next
    worker's state.
  - `worker_body` no longer clears the flag internally (the
    generation-guarded tail owns it).
- Added the `clear_flag_if_current` guard with a unit test proving stale
  generations cannot clobber the flag.
- Accepted trade-offs: a pathological portal hang leaks the worker thread
  (the UI keeps working); a stop→start within one portal-click duration
  can land one in-flight click from the old worker.
- Verified: `cargo test --release` (4 tests pass) and `cargo build --release`.
- Doc note: ARCHITECTURE.md still says "stop signals and joins it" — reword
  in the final doc rewrite.
