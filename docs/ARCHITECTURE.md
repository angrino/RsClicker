# Architecture

## Modules

| Path                   | Responsibility                                              |
| ---------------------- | ----------------------------------------------------------- |
| `main.rs`              | Entry point; panic-hook filter; Linux renderer default      |
| `app.rs`               | `RsClicker` state, `Message`, `update`, subscriptions, view |
| `theme.rs`             | `BaseTheme`/`Accent` × palettes, shared widget styles      |
| `trigger.rs`           | `EffectiveTrigger`, key capture, armed routing helpers      |
| `mouse.rs`             | Global mouse monitor (platform backends below)              |
| `clicker/engine.rs`    | One worker thread per running profile, 50 ms sleep slices   |
| `clicker/types.rs`     | `ClickConfig`, `ClickTarget`, enigo glue                    |
| `hotkey/`              | `HotkeyService` wrapper around global-hotkey                |
| `profile/`             | `Profile` model + JSON file store                           |
| `settings_store.rs`    | Persisted `AppSettings` (`settings.json`)                  |
| `views/`               | `splash`, `main` (list/grid), `editor` modal, `dialogs`     |

## Runtime model

- **Engines:** `HashMap<String, ClickEngine>` — at most one worker thread
  per profile. `start` spawns (or restarts) the thread; `stop` signals it,
  waits at most 50 ms on a per-worker exit signal, and hands any leftover
  join to a detached `rsclicker-join` reaper — the UI thread never joins.
  Per-engine generation counters guard state tails from stale workers.
  Clicks follow a shared deadline train (`Instant`-anchored, drift-free);
  sleeps stay in ≤ 50 ms slices so stop latency stays low even for
  hour-long intervals.
- **Armed triggers:** `HashMap<String, ArmedProfile>` — keyboard triggers
  are registered globally (`trigger_id → profile`), mouse triggers need no
  registration and are matched from the monitor channel. Global hotkey
  events are not routed while the Capture modal is open (drained, then
  discarded).
- **Event flow:** every 50 ms tick drains (1) global-hotkey press/release
  events, (2) the mouse-monitor channel, (3) per-engine error flags.
  Toggle uses press edges; Hold uses press + release.
- **Errors:** worker backends report through `Arc<Mutex<Option<String>>>`
  (`take_error`), surfaced as red ERROR states — never a crash.

## Platform backends

| Capability      | Linux (Wayland)              | Linux (X11)              | Windows 10/11        |
| --------------- | ---------------------------- | ------------------------ | -------------------- |
| Global hotkeys  | portal `GlobalShortcuts`     | X11 protocol             | Win32 (native)       |
| Clicking        | portal `RemoteDesktop`/libei, or uinput | XTEST via x11rb, or uinput | SendInput (native) |
| Mouse triggers  | evdev (`input` group)        | evdev (`input` group)    | rdev hook (native)   |
| Key capture     | iced (in-app focus)          | iced (in-app focus)      | iced (in-app focus)  |

Linux clicks go through either enigo (default: XTEST on X11, the
`RemoteDesktop`/libei portals on Wayland) or the settings-switchable
Kernel-level uinput backend (`/dev/uinput`, needs the `uinput` group; the
virtual device's clicks surface in `/dev/input`, where the injected-pair
filter drops them).

Minimal Wayland compositors without the portals above will show red ERROR
states for hotkeys/clicking — by design, instead of panicking. Mouse
triggers via evdev keep working there.

## Profiles on disk

`{config}/rsclicker/profiles/<sanitized>.json`, pretty JSON. Field renames
keep `serde(alias)` compatibility (e.g. `toggle_hotkey` → `trigger_hotkey`);
unknown keys are ignored. Renaming a profile is save-new + delete-old and
only when the sanitized file name actually changes — live armed state is
rebuilt onto the new key by the resync step. Sanitize collisions
(`a/b` vs `a:b`) are rejected at create/rename time.
