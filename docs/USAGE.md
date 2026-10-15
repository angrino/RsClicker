# Usage

## Profiles

- The main screen lists all saved profiles (stored as JSON under the OS
  config dir, e.g. `~/.config/rsclicker/profiles/` on Linux,
  `%APPDATA%\rsclicker\profiles\` on Windows).
- **+** (bottom-left) creates a profile: enter a name, Confirm.
- Click a profile name or **⚙** to open its editor (a modal dialog).
- The editor footer has **Save**, **Delete**, and manual **Start/Stop** for
  testing the current settings.

## Arming and running

Each row has a run button with four states:

| Icon  | Color  | Meaning                              |
| ----- | ------ | ------------------------------------ |
| ▶     | gray   | Idle — press to arm (start listening) |
| ●     | orange | Listening — press your trigger to run |
| ■     | green  | Running — clicking right now          |
| ⚠     | red    | Error — press to clear, see status    |

Pressing the button on an armed profile disarms it. Several profiles can be
armed and running at the same time; the status line always shows the latest
event.

## Triggers

In the editor's **Trigger** section you see the current trigger
(`F6` or `Mouse Side`, …) and a **Capture** button. Capture opens a small
dialog saying *"Press the key you want to assign"*:

- **Keyboard and mouse are listened together** — press either one.
- Combos work: hold `Control`/`Shift`/`Alt`/`Super` and press the main key
  (`Control+R`, `Shift+F6`, …).
- Mouse side buttons (`Side`, `Extra`) are captured from the real device.
- `Esc` cancels. Unusable keys show a hint and keep listening.

Typing into the text field manually sets a keyboard hotkey (this clears any
mouse trigger).

### Press modes

- **Toggle** — press once to run, press again to stop.
- **Hold** — runs only while the trigger is held down, stops on release
  (uses both press and release events).

## Timing

Enter a number and pick a unit: `ms`, `s`, `min` — e.g. `2 ms` or `0.7 s`.
Stored as milliseconds (minimum 1 ms). Invalid input shows a status error
and keeps the previous value.

## Click and target

- **Button:** Left / Right / Middle. **Type:** Single / Double.
- **Target modes:**
  - *Current* — clicks wherever the cursor is.
  - *Fixed* — clicks at fixed X/Y coordinates (invalid input is rejected
    with a message; the last valid value is kept).
  - *Sequence* — cycles through added points; arming requires at least one
    point (`Add`/`Clear` manage the list).

## Themes and settings

The top-bar **⚙** opens app settings:

- **Theme:** Light / Dark cards, **Accent:** 8 swatches (Orange, Aqua,
  Green, Purple, Red, Yellow, Pink, Blue).
- **Profiles view:** List / Icons.
- **About** — project description + repository link; **License** shows the
  full MIT text.
- Changes apply only after **Apply** (**Cancel** discards); settings persist
  in `settings.json` next to the profiles.
