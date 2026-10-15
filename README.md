# RsClicker

A cross-platform auto clicker written in Rust.

Create profiles, arm them with a keyboard hotkey or a mouse button (including
side buttons), and fire precise click sequences anywhere. Built with
[iced](https://iced.rs) for the GUI, [enigo](https://github.com/enigo-rs/enigo)
for input emulation, and raw input monitoring (evdev on Linux, rdev on
Windows 10/11) for global mouse triggers.

- Repository: <https://github.com/voidx9x/RsClicker>
- Author: voidx9x
- License: MIT (see [LICENSE](LICENSE))
- Release: **26.10.15** (date-based, see [docs/VERSIONING.md](docs/VERSIONING.md))

## Features

- **Profiles** — save named click configurations (interval, button, target);
  list or icon-grid views.
- **Triggers** — keyboard hotkeys (`F6`, `Control+R`…) or mouse buttons
  (left/right/middle/side/extra), captured by simply pressing them.
- **Press modes** — Toggle (press to run, press to stop) or Hold (runs while
  held, stops on release).
- **Targets** — current position, fixed coordinates, or a point sequence.
- **Multi-run** — arm and run several profiles at the same time; per-profile
  state colors: gray idle, orange listening, green running, red error.
- **Themes** — Light/Dark base × 8 accent colors, applied via settings.
- **Animated splash** with brand intro, modal dialogs, settings persisted to
  disk.

## Quick start

```bash
cargo build --release
./target/release/RsClicker
```

1. Press **+** to create a profile.
2. Open it with **⚙**, set timing/clicks/target, then **Capture** your trigger.
3. Back on the main screen press **▶** — it turns orange (listening).
4. Press your trigger key/button: green means clicking. Press again to stop.

Details in [docs/USAGE.md](docs/USAGE.md). Build instructions (Linux +
Windows cross-compilation) in [docs/BUILDING.md](docs/BUILDING.md).

## Requirements

- **Linux:** Wayland or X11. Global hotkeys need
  `org.freedesktop.portal.GlobalShortcuts`; clicking on Wayland needs
  `org.freedesktop.portal.RemoteDesktop` (GNOME/KDE/Hyprland provide these;
  minimal compositors may not — the app reports this as a red ERROR state
  instead of crashing). Mouse triggers need read access to `/dev/input`
  (membership in the `input` group).
- **Windows 10/11:** works out of the box (Win32 hotkeys, hooks, SendInput).

## Project layout

```
src/
  main.rs            entry point
  app.rs             state, messages, update, subscriptions
  theme.rs           Light/Dark x 8 accents, shared widget styles
  trigger.rs         trigger model, key capture, armed routing
  mouse.rs           global mouse monitor (evdev / rdev)
  clicker/           click engine (own thread per profile)
  hotkey/            global-hotkey service wrapper
  profile/           profile model + JSON store
  settings_store.rs  persisted app settings
  views/             splash, main, editor modal, dialogs
docs/                usage, architecture, building, versioning
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Credits

Developed with the [opencode](https://opencode.ai) coding tool, with human
touches (design decisions, testing on real hardware, all product choices) by
**voidx9x**. Licensed under MIT.
