# Features

The full feature set of RsClicker, with what is new in **26.10.19** marked.
For history of every fix, see [FIXLOG.md](FIXLOG.md).

## Core

- **Profiles** — save named click configurations (interval, button, click
  type, target) as JSON under the OS config dir; list or icon-grid views.
- **Triggers** — keyboard hotkeys (`F6`, `Control+R`…) or mouse buttons
  (left/right/middle/side/extra), captured by simply pressing them in the
  editor (keyboard + mouse listened together, `Esc` cancels).
- **Press modes** — Toggle (press to run, press again to stop) or Hold
  (runs while the trigger is held, stops on release).
- **Targets** — current cursor position, fixed X/Y coordinates, or a cycling
  point sequence (`Add`/`Clear` manage the list; arming requires at least
  one point).
- **Multi-run** — arm and run several profiles at the same time; per-profile
  state colors: gray idle, orange listening, green running, red error.
- **Themes** — Light/Dark base × 8 accent colors; profiles view switch;
  everything persisted to `settings.json`.
- **Animated splash**, modal dialogs (new profile, editor, settings,
  About/License), status line reporting the latest event.

## NEW in 26.10.19

### Linux injection backend: kernel-level uinput
A second, settings-switchable Linux clicking backend (Settings → **Input
emulation**). It creates a kernel-level virtual mouse via `/dev/uinput` —
no X11 server and **no desktop portals** involved, so it works everywhere
the portals misbehave (X11, Wayland, whatever compositor). Fixed/sequence
targets are kept exact by corner-clamp differential moves; failures (e.g.
missing permission) surface as a red ERROR state with the fix hint, and
remain re-armable. Requires the `uinput` group:

```bash
sudo usermod -aG uinput $USER   # log out and back in afterwards
```

enigo (XTEST on X11, desktop portals on Wayland) stays the default.

### Drift-free click cadence
Clicks now follow a shared deadline train anchored with `Instant` — the
configured interval is kept even when a click costs a long time (slow
backends, deep sleep), with no catch-up bursts. A 40 ms interval now
produces a hall click every ~40 ms.

### Real double clicks
Double-click profiles now interleave a 30 ms human-like gap
(`DOUBLE_CLICK_GAP_MS`) between the two clicks — X11, Wayland compositors
and Windows recognize both clicks instead of folding them into one.

### Safer saves and renames
- Saving never deletes the profile file: renames are detected by the
  sanitized file name, not the display string — a display name like
  `My Profile` (kept on disk as `My_Profile.json`) survives every save.
- Armed state resyncs silently after a failed save edit; a real
  rename/rearm carries state to the new key, and a failed rebuild disarms
  with the reason shown.

### Capture-mode isolation
While the trigger Capture dialog is open, armed global hotkeys no longer
route — capturing F6 with an F6-armed profile toggles nothing.

### Robust engine and file handling (carried over from earlier sessions)
- Worker panics inside the click loop can't leave a zombie "Running" state;
  injection failures stop the engine and surface the reason.
- A dedicated reaper keeps stop/start responsive (the UI never joins a
  dying worker).
- Profile/settings saves are atomic — a crash mid-write never truncates
  the JSON; a self-healing mouse monitor respawns dead readers and picks up
  hot-plugged devices.
