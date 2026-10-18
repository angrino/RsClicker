# FIXLOG

Working log of the important fixes. This file is a temporary record — it
will be rewritten into the final documentation when the next release is
cut. Each session's fixes land under one dated session heading.

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

### Fixed (high): stale armed state after saving edits
- Symptom: `ArmedProfile` was a snapshot taken at arm time (config,
  trigger, press mode). Editing and saving an armed profile updated the
  file while the old trigger kept routing — e.g. changing F6 to F7 and
  saving still left F6 firing, and interval/press-mode/points edits were
  invisible to the live engine.
- Fix (live-resync, as agreed): armed matching state is preserved
  silently; anything else is rebuilt after the save.
  - `src/trigger.rs`: `armed_matches_profile` — pure comparison of the
    armed snapshot against the profile (same trigger id — hotkeys are
    re-parsed, so an unparseable edit never matches — plus press mode and
    click config, where an unusable config never matches).
  - `src/app.rs`: arming resolution is factored into
    `resolve_effective_trigger` (monitor gate, duplicate checks,
    registration — identical user-visible reason strings), and
    `resync_after_save` runs after a successful save: unchanged state is
    left untouched ("Saved <name>"), a changed armed profile is rebuilt
    (trigger re-registered, running engine restarted with the new
    config, "Saved <name> — armed state updated.") and renames carry the
    armed state under the new key. A failed rebuild (conflicting
    trigger, unusable hotkey, empty sequence, missing monitor) cleans up
    to Idle and reports the reason: "Saved <name>. Disarmed: <reason>".
- Manual Start/Stop engines from the editor keep their explicit snapshot
  semantics (unchanged by design).
- Unit tests: snapshot equality + edit matrix (interval, hotkey, press
  mode, click button, trigger button) + unusable-state matrix
  (unparseable hotkey, empty sequence) for `armed_matches_profile`.

### Session verification
- `cargo test --release`: 15 tests pass (engine 6, mouse monitor 1,
  atomic-file 2, reserved names 3, trigger resync 3).
- `cargo build --release`: clean.

## Session 2 — 2026-10-17

### Fixed (high): self-trigger feedback loop when the trigger button equals the click button
- Symptom: a profile armed with a mouse trigger identical to its click
  button was destroyed by its own clicks. On Windows the rdev hook is a
  `WH_MOUSE_LL` low-level hook, which receives the app's own injected
  `SendInput` clicks like any real input (rdev does not expose
  `LLMHF_INJECTED`), so: **Toggle** stopped the engine at the first
  injected click (the injected *press* edge toggled it off) and **Hold**
  stopped it instantly (the injected *release* edge arrived while the
  trigger was held). One click, then dead — or a stop/start feedback
  flicker at rapid intervals. Linux XTEST/libei injection never re-enters
  `/dev/input`, so no live Linux loop exists today; the fix is still
  cross-platform and future-proofs any uinput-style Linux backend
  (e.g. a re-added enigo `wayland` feature).
- Fix (`src/mouse.rs`, drain in `src/app.rs`):
  - `MouseEvent::Press/Release` now carry the device timestamp (millis
    since the UNIX epoch): evdev uses `InputEvent::timestamp()` (same
    timeval for a click's press+release), rdev uses the per-event
    `Event.time = SystemTime::now()` stamped inside the hook callback.
  - New pure function `filter_injected_pairs`: a same-button
    press→release pair with a gap ≤ `PAIR_EPS_MS` (20 ms) is an injected
    click fingerprint — SendInput pairs land ≤ ~2 ms apart, evdev pairs
    share a timeval, while the fastest physical release stays far above
    the bound. Both edges are dropped together — a release only cancels
    its latest *unmatched* press, so held-button semantics still route
    normally.
  - The Tick handler drains the monitor channel into a batch, filters,
    then routes/captures the survivors — so self-injected clicks can
    never re-trigger an armed profile, and a running engine cannot pollute
    a concurrent Capture session.
- Accepted edge: a physical release within 20 ms of its press (jitter
  clickers/fast macros) is indistinguishable from injection and is
  dropped too; split pairs across two drain batches are theoretically
  possible but practically impossible for near-instant injection pairs.
- Added 10 portable unit tests (`mouse::filter_tests`): instant pair,
  double-click pairs, slow pair survives, release-before-press never
  pairs, stale long press intact, unpaired held press, cross-button
  independence + ordering, held press with concurrent other-button pair,
  and slow pair followed by a fast pair.

### Minor batch: poisoned-mutex consistency + cosmetic leftovers (#11)
- `ClickEngine::take_error()` now opens its slot with
  `unwrap_or_else(|e| e.into_inner())` (was `lock().ok()?`), matching the
  rest of the codebase — a poisoned slot no longer drops a pending error.
  `MouseMonitor::status()` got the same unlock-on-poison style instead of
  silently falling back to `Unknown`.
- Dead code removed: `hotkey/binding.rs` keeps only `parse_hotkey`
  (`describe_modifiers`, `example_keys`, `default_toggle`,
  `hotkey_to_string` deleted together with their `#[allow(dead_code)]`
  markers), and `HotkeyService::available()` is gone.
- `Message::Stop` no longer overwrites the status unconditionally: it
  stamps "Stopped." only when the engine was actually running.
- `new()` no longer swallows a corrupt default profile: a missing
  `default.json` stays the normal first-run path, but a present file
  that fails to parse shows "default profile failed to load: …" at
  startup (built-in defaults are used meanwhile).
- Renaming a profile surfaces a failed delete of the old file instead of
  silently orphaning it: "Saved <new> (old file kept: <reason>)".
- ApplySettings keeps the user's in-session choice on a failed save and
  says so ("Settings kept in memory — save failed: …").
- Interval input: `MAX_INTERVAL_MS = 3_600_000` is now the single shared
  bound (used by `ClickConfig::sanitized_interval_ms` and the UI). New
  pure `resolve_interval` replaces the open-coded parsing: empty input
  means "no change", tiny positives still clamp to 1 ms, and values above
  60 minutes are **rejected** with "Interval too large — maximum is
  60 min" while the previous value stays — `"1e300"` can no longer store
  `u64::MAX` into a profile file. 6 unit tests added.
- The `engines` map can no longer grow unbounded: `drain_engine_errors`
  retains only running engines and prunes idle ones (an engine that just
  reported an error is surfaced first and pruned on the next tick). Since
  state colors derive from the `errors` and `armed` maps, nothing
  user-visible changes.

### Session verification
- `cargo test --release`: 31 tests pass (engine 6, mouse 11, atomic-file
  2, reserved names 3, trigger resync 3, interval 6).
- `cargo build --release`: clean.
- `cargo check --target x86_64-pc-windows-gnu --release`: clean.

## Session 3 — 2026-10-18

### Fixed (critical): saving kept a stale display name → profile file deleted
- Symptom: profiles are listed and keyed by their sanitized file stem
  (`My Profile` is stored as `My_Profile.json` and listed as
  `My_Profile`). Saving such a profile from the editor compared the raw
  strings — `"My Profile" != "My_Profile"` — so `SaveProfile` treated
  every save as a rename: it wrote `My_Profile.json` (same file), then
  `delete_profile("My_Profile")` deleted the file just written. The
  profile vanished from disk on the first Save; armed state pointed at a
  key the list no longer shows. Found by the session-3 self-audit (user
  asked for undocumented bugs), not user-reported.
- Fix (`src/app.rs`):
  - `SaveProfile` decides "rename" with `same_profile_file(old, new)` —
    sanitized comparison — instead of raw string inequality; the
    duplicate-name check now only fires when the target file actually
    differs from the old one (`editing_name` also only moves on a real
    rename, so the editor's manual-engine key stays the list stem).
  - `resync_after_save` treats a save whose display name sanitizes onto
    the same file as a same-key save: armed/engines/errors keep their
    list key (no spurious rebuild/rearm when the config is unchanged,
    rebuild stays under the stem key when it is needed). Real renames
    (different sanitized file) behave exactly as before: state moves to
    the new key, the old file is deleted.
- Tests: `sanitize_collision_matrix` extended with the stem-vs-display
  cases (`"My Profile"` vs `"My_Profile"`, `"Nice profile!"` vs
  `"Nice_profile_"`).

### Fixed (high): armed hotkeys routed while the Capture modal was open (#13)
- Symptom: `Message::Tick` polled and routed global-hotkey events
  unconditionally, so an armed profile kept firing while the Capture
  modal recorded a new trigger — pressing F6 to capture F6 toggled the
  F6-armed profile instead of being sampled (mouse events already gated
  capture vs routing; hotkeys did not). Backlog issue #13.
- Fix (`src/app.rs`): `hotkeys_route_allowed(top_modal)` gates
  `route_hotkey`; while capture is open the hotkey queue is still drained
  so events don't pile up, but every edge is discarded. The capture flow
  itself is unaffected (keyboard capture runs via `Message::KeyEvent`,
  which keeps its own modal gate).
- Tests: `app::hotkey_gate_tests` — capture suppresses, other modals and
  `None` route.

### Session verification
- `cargo test --release`: 33 tests pass (31 previous + 2 hotkey-gate).
- `cargo build --release`: clean.
- `cargo check --target x86_64-pc-windows-gnu --release`: clean.
