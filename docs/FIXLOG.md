# FIXLOG

Release history in one file: what changed, where the fix lives, what is
still accepted as a known limit. New sessions append under a release
heading. Deep-dive details for the current release live in
[FEATURES.md](FEATURES.md) / [ARCHITECTURE.md](ARCHITECTURE.md).

- Symbols: `(critical/high/medium/minor)` importance, `#n` backlog issue.
- Every session ends with the release gate: `cargo test --release`,
  `cargo build --release`, and a clean
  `cargo check --target x86_64-pc-windows-gnu --release`.

## 26.10.15 — initial public release

Baseline: profiles, keyboard/mouse triggers with press-to-capture, Toggle /
Hold modes, monotonic timing, fixed/sequence targets, multi-run engines,
themes, settings/profiles stored as JSON, evdev/rdev mouse-trigger monitors,
iced GUI with animated splash.

## 26.10.19 — current

### New: kernel-level uinput injection backend (Linux, optional)
- enigo stayed portal-bound on some Wayland setups; users' distros
  misbehaving need a portal-free path.
- `clicker/types.rs`: `ClickBackend` trait + `UinputBackend` — evdev
  `VirtualDevice` (BTN_* + REL_X/Y) writing straight to the kernel; exact
  Fixed/Sequence positioning via corner-clamp deltas (−32767 to the
  top-left corner, exact positive offset after); dropping the backend
  destroys the virtual device.
- `clicker/engine.rs`: engines pick the backend at `start` — workers fail
  through `StartError::NoBackend` (enigo catch_unwind path) or
  `UinputUnavailable` ("add your user to the uinput group") while staying
  re-armable.
- `settings_store.rs`: `input_backend` setting (serde default → enigo);
  Windows always enigo/SendInput and the settings row is Linux-only.
- Self-trigger safety: uinput clicks surface in `/dev/input`, but the
  injected-pair filter (below) already drops them. Real-hardware validation
  is the last open check.

### New: fixed cadence + true double clicks
- Old loop slept the full interval after each click, so real cadence was
  `interval + click latency` (up to ~50% drift). Now the loop follows an
  `Instant` deadline train; a missed deadline resets to `now + interval`
  (no catch-up bursts).
- Double clicks got a 30 ms inter-click gap (`DOUBLE_CLICK_GAP_MS`), so
  X11/Wayland/Win32 no longer fold the pair into one click.

### Fixed (critical): saving could delete the profile file
- Profiles are keyed by their sanitized file stem (`My Profile` ⇔
  `My_Profile.json`); save compared raw strings, so editing such a profile
  wrote the file and then deleted it as a phantom "rename".
- Fix (`app.rs`): renames are detected via `same_profile_file`
  (sanitized), live state keeps the list stem as its key, and real renames
  behave as before (save-new + delete-old + armed-state resync).

### Fixed (high): armed hotkeys fired during Capture (#13)
- The Capture dialog used to route armed global hotkeys — capturing F6
  while an F6 profile was armed toggled it mid-capture.
- Fix (`app.rs`): `hotkeys_route_allowed` gates routing; events are still
  drained, then discarded, while capture is open.

### Fixed (high): self-trigger feedback loop
- Windows `WH_MOUSE_LL` receives injected clicks like real input, so a
  profile armed with a trigger equal to its click button killed itself
  (Hold died instantly, Toggle flickered).
- Fix (`mouse.rs` + drain in `app.rs`): press→release pairs ≤ 20 ms apart
  (`PAIR_EPS_MS`) are treated as injected and dropped wholesale, at every
  routing and capture path.
- Accepted edge: the fastest physical clicks can also fall inside the
  20 ms window (rare, humanly indistinguishable from software clicks).

### Fixed (high): zombie RUNNING state after a worker panic
- A panic inside the click loop left the flag raised forever (green with
  no clicks, no message).
- Fix (`clicker/engine.rs`): the worker body runs inside `catch_unwind`;
  crashes record "Engine crashed: …" and clear the flag.

### Fixed (high): UI freeze on stop (join on the UI thread)
- `stop()` joined the worker inline — a hung portal call froze the UI
  indefinitely.
- Fix (`clicker/engine.rs`): stop signals, waits ≤ 50 ms on a per-worker
  exit signal, then a detached `rsclicker-join` reaper finishes the join;
  generation counters keep stale tails from clobbering the next worker.

### Fixed (high): silent injection failures (green, zero clicks)
- Every enigo result was discarded; a denied portal clicked nothing while
  staying green.
- Fix (`clicker/types.rs` + engine loop): the first injection failure
  stores "Click failed: …" and exits — red state, profile re-armable;
  the first click of a refused backend also fail-fasts.

### Fixed (high): silent, dead, or blind mouse-trigger monitor
- Broken `/dev/input` accesses passed silently; dead reader threads never
  recovered; hot-plugged mice were never seen; Windows hook failure was
  silent too.
- Fix (`mouse.rs`): `MonitorStatus` (Unknown/Running(n)/Unavailable)
  surfaced in the status line on arming; a 5 s scanner respawns dead
  readers, picks up new devices and self-detects availability; Windows
  records hook failures.

### Fixed (high): non-atomic profile/settings saves
- Plain `fs::write` could truncate JSON mid-crash or disk-full; a killed
  save also silently reset the settings.
- Fix (`atomic_file.rs`): `.tmp` sibling + flush + atomic rename
  (`MOVEFILE_REPLACE_EXISTING` on Windows) — used by both savers.

### Fixed (high): Windows reserved device names ate saves
- `NUL.json`, `COM1.json`… are DOS devices even with an extension — saves
  reported success while the file never existed.
- Fix (`profile/store.rs`): superscript digits normalize; reserved bases
  get a trailing `_` universally (`NUL_.json`).

### Fixed (high): stale armed state after saving edits
- Veterans ran the old snapshot: editing an armed profile kept firing the
  old trigger/config while the file showed the new one.
- Fix (`trigger.rs` + `app.rs`): unchanged state survives saves untouched;
  changed armed profiles rebuild live (re-register, restart into the new
  config, carry renames), and failed rebuilds disarm with the reason.

### Minor batch
- Poisoned-mutex consistency (`take_error`, monitor status), a windows
  reserved-name test matrix, engines pruned when idle so the map cannot
  grow unbounded, interval input hard stop at 60 min with a pure
  `resolve_interval` (rejects `"1e300"`), failed settings saves keep the
  in-session choice, deleting a profile surfaces a failed delete of the
  old file instead of silently orphaning it.
- Hotkey binding helpers trimmed to what the app uses.

## Known open items

- uinput backend needs one real-hardware pass (group permission + loop
  behavior + portal-less Wayland session).
- Versioned docs for profile files: no schema version yet — new fields
  must keep `serde(default)`.
