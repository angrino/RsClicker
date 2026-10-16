# FIXLOG

Working log of the important fixes. This file is a temporary record — it
will be rewritten into the final documentation when the next release is
cut. All fixes from the current working session land under one session
heading.

## Session 1 — 2026-10-16

### Docs audited
- `docs/*.md` + `README.md` checked against the code: accurate, with two
  wording notes (a profile rename is a save-new + delete-old, not an
  atomic move, and stop no longer "joins it" — see below). Reword both in
  the final doc rewrite.

### Fixed (critical): zombie RUNNING state after a worker panic
- Symptom: a panic inside the click loop (enigo internals, e.g. the libei
  portal path) killed the worker thread while the running flag stayed
  `true` — the engine showed green "Running" forever, no clicks, no error,
  and the panic hook in `main.rs` swallowed the message.
- Fix (`src/clicker/engine.rs`): the whole worker body (enigo creation +
  click loop) is wrapped in `catch_unwind`; a crash records
  `Engine crashed: <message>` in the engine error slot and clears the
  running flag → the UI's per-tick drain surfaces the red ERROR state and
  the profile stays re-armable. Added `panic_message()` helper with unit
  tests.

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

### Fixed (critical): silent injection failures (green running, zero clicks)
- Symptom: every enigo call in `perform_single_click` discarded its result
  (`let _ =`), so a denied portal, a dead session or a broken backend left
  the engine green "Running" with no clicks and nothing in the error slot.
- Fix:
  - `src/clicker/types.rs`: `perform_single_click` now returns
    `Result<(), String>`; `move_mouse` and `button` failures abort the
    click (fail-fast — clicking in a wrong place is worse than not
    clicking), and an empty `Sequence` target is rejected defensively.
  - `src/clicker/engine.rs`: the click loop is factored into
    `run_click_loop`; the first injection failure records
    `Click failed: <reason>` in the engine error slot (only while the
    worker is still the current generation) and exits the loop — the
    existing generation-guarded tail clears the flag, so the UI surfaces
    the red ERROR state and the profile stays re-armable.
- Added two deterministic unit tests for the new loop semantics (no enigo
  needed: fail-fast reporting, and stale-generation errors being dropped).

### Fixed (high): silent mouse-trigger monitor (no feedback, dead readers, no hot-plug)
- Symptom: `evdev::enumerate()` swallows its own errors (verified from the
  crate source), so an unreadable `/dev/input` passed silently; a reader
  thread dying on its first error was gone forever; devices plugged in
  after startup were never monitored; on Windows a failed `rdev::listen`
  hook stayed silent.
- Fix (`src/mouse.rs`, arming gate in `src/app.rs`):
  - `spawn_monitor` returns a `MouseMonitor` carrying a shared
    `MonitorStatus` (`Unknown` / `Running(n)` / `Unavailable(reason)`).
  - Linux: a permanent scanner re-enumerates every 5 s and respawns dead
    readers (`reader_wanted` dedupe + per-reader alive flags) — dead
    readers and hot-plugged devices self-heal within one rescan;
    availability is self-detected (`read_dir` + readable mouse count),
    never taken from evdev's silent errors.
  - Windows: a failed hook records `Unavailable("mouse hook failed: …")`.
  - `toggle_arm` refuses to arm a mouse trigger while the monitor is
    `Unavailable` and shows the reason in the status line; arming during
    the first-scan `Unknown` window stays allowed; no proactive status
    noise at startup.
- Added a deterministic unit test for the reader respawn lifecycle.

### Fixed (high): non-atomic file writes (JSON corruption on crash mid-save)
- Symptom: `save_profile` and `save_settings` used plain `fs::write`
  (open + truncate + write) — a kill/crash/disk-full mid-write left a
  truncated JSON file behind permanently (broken profile, or settings
  silently reset to defaults on load).
- Fix: new `src/atomic_file.rs` — `write_atomic(path, data)` writes a
  `.tmp` sibling, flushes (`sync_all`), then atomically renames over the
  target (POSIX semantics; `MOVEFILE_REPLACE_EXISTING` on Windows; file
  closed before rename). Both savers use it. Leftover `.tmp` files from a
  crash are invisible to the profile listing and overwritten by the next
  save.
- Added unit tests: replace-existing content and stale-tmp resilience.
- Accepted limit: full power-loss durability (directory fsync) is out of
  scope; the threat model is process kill / crash / disk-full.

### Fixed (high): Windows reserved device names silently eat saves
- Symptom: on Win10/11 (pre-Win11 rule kept for the 10/11 targets)
  `NUL.json`, `CON.json`, `COM1.json`… are DOS devices even with an
  extension — `fs::write` reported success while the file went nowhere: a
  profile was "created" but never listed, never saved. `sanitize_name`
  already rejected dots and spaces, but kept these bases (and superscript
  digits like `COM¹` pass `char::is_alphanumeric`, so they survived too).
- Fix (`src/profile/store.rs`): `is_reserved_name` (case-insensitive,
  superscript digits normalized to ASCII) + `sanitize_name` appends `_`
  to reserved bases on every platform (universal rule, as agreed) →
  `NUL` is stored as `NUL_.json`, which is a safe regular file.
- Unit tests: reserved bases (lower case, mixed case, `COM¹`) get the
  suffix; plain names (`COM10`, `COM¹²`, `nul_`) untouched; the
  `a/b` vs `a:b` collision matrix unchanged.
- Accepted edge: legacy profiles already saved under a reserved name on
  Linux (rare) stay listed but fail to load — consequence of the
  universal rule.

### Session verification
- `cargo test --release`: 12 tests pass (engine 6, mouse monitor 1,
  atomic-file 2, reserved names 3).
- `cargo build --release`: clean.
