use std::collections::HashMap;
use std::time::Duration;

use global_hotkey::HotKeyState;
use iced::widget::{opaque, stack};
use iced::{keyboard, Element, Subscription, Task};

use crate::clicker::{ClickConfig, ClickEngine, ClickTarget};
use crate::hotkey::{parse_hotkey, HotkeyService};
use crate::mouse::{
    filter_injected_pairs, spawn_monitor, MonitorStatus, MouseMonitor, MouseEvent, MouseTrigger,
};
use crate::profile::{
    delete_profile, list_profiles, load_profile, profile_file_exists, same_profile_file,
    save_profile, split_interval, ClickPoint, PressMode, Profile, TimeUnit,
};
use crate::settings_store::{load_settings, save_settings, AppSettings};
use crate::theme::{Accent, BaseTheme, ViewMode};
use crate::trigger::{
    armed_matches_profile, capture_hotkey, profile_config, trigger_label, ArmedProfile, Capture,
    EffectiveTrigger, RunState,
};
use crate::views::{self};

/// Project release version (date-based YY.MM.DD, see
/// `[package.metadata.rsclicker]` in Cargo.toml). Note: Cargo package
/// version intentionally stays at 0.1.0.
pub(crate) const VERSION: &str = "26.10.15";
pub(crate) const REPO: &str = "https://github.com/voidx9x/RsClicker";
pub(crate) const SPLASH_TICKS: u32 = 44; // ~2.2s at 50ms per tick

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Splash,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modal {
    NewProfile,
    Editor,
    Capture,
    AppSettings,
    About,
    License,
}

pub fn run() -> iced::Result {
    iced::application(new, update, view)
        .title("RsClicker")
        .window_size(iced::Size::new(480.0, 700.0))
        .theme(|state: &RsClicker| state.settings.theme())
        .subscription(subscription)
        .run()
}

pub struct RsClicker {
    pub(crate) screen: Screen,
    pub(crate) settings: AppSettings,
    pub(crate) pending_settings: AppSettings,
    pub(crate) modal: Vec<Modal>,
    pub(crate) modal_alpha: f32,
    pub(crate) splash_tick: u32,
    pub(crate) new_profile_name: String,
    pub(crate) dialog_error: String,
    pub(crate) editing_name: Option<String>,
    pub(crate) profile: Profile,
    pub(crate) engines: HashMap<String, ClickEngine>,
    pub(crate) armed: HashMap<String, ArmedProfile>,
    pub(crate) errors: HashMap<String, String>,
    pub(crate) hotkeys: HotkeyService,
    pub(crate) mouse: MouseMonitor,
    pub(crate) interval_value: String,
    pub(crate) interval_unit: TimeUnit,
    pub(crate) fixed_x: String,
    pub(crate) fixed_y: String,
    pub(crate) point_x: String,
    pub(crate) point_y: String,
    pub(crate) profiles: Vec<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SkipSplash,
    KeyEvent(keyboard::Event),
    OpenNewProfile,
    NewProfileNameChanged(String),
    ConfirmNewProfile,
    CloseModal,
    OpenEditor(String),
    DeleteProfile,
    OpenAppSettings,
    ApplySettings,
    SetTheme(BaseTheme),
    SetAccent(Accent),
    SetViewMode(ViewMode),
    OpenAbout,
    OpenLicense,
    OpenCapture,
    ToggleArm(String),
    IntervalValueChanged(String),
    IntervalUnitSelected(TimeUnit),
    HotkeyChanged(String),
    PressModeSelected(PressMode),
    ButtonSelected(crate::clicker::MouseButtonKind),
    KindSelected(crate::clicker::ClickKind),
    TargetCurrent,
    TargetFixed,
    TargetSequence,
    FixedXChanged(String),
    FixedYChanged(String),
    PointXChanged(String),
    PointYChanged(String),
    AddPoint,
    ClearPoints,
    ProfileNameChanged(String),
    SaveProfile,
    Start,
    Stop,
}

fn new() -> (RsClicker, Task<Message>) {
    let settings = load_settings();
    let loaded = load_profile("default").unwrap_or_default();
    let (interval_value, interval_unit) = split_interval(loaded.interval_ms);
    let mut app = RsClicker {
        screen: Screen::Splash,
        settings,
        pending_settings: settings,
        modal: Vec::new(),
        modal_alpha: 0.0,
        splash_tick: 0,
        new_profile_name: String::new(),
        dialog_error: String::new(),
        editing_name: None,
        profile: loaded,
        engines: HashMap::new(),
        armed: HashMap::new(),
        errors: HashMap::new(),
        hotkeys: HotkeyService::new(),
        mouse: spawn_monitor(),
        interval_value,
        interval_unit,
        fixed_x: "500".to_string(),
        fixed_y: "300".to_string(),
        point_x: "500".to_string(),
        point_y: "300".to_string(),
        profiles: list_profiles(),
        status: "Ready".to_string(),
    };
    app.refresh_fixed_inputs();
    app.apply_view();
    (app, Task::none())
}

impl RsClicker {
    pub(crate) fn view_mode(&self) -> ViewMode {
        self.settings.view
    }

    fn apply_view(&mut self) {
        // view mode is read live from settings; nothing to cache.
    }

    pub(crate) fn run_state(&self, name: &str) -> RunState {
        if self.errors.contains_key(name) {
            RunState::Error
        } else if self
            .engines
            .get(name)
            .map(|e| e.is_running())
            .unwrap_or(false)
        {
            RunState::Running
        } else if self.armed.contains_key(name) {
            RunState::Listening
        } else {
            RunState::Idle
        }
    }

    /// Engine key for the editor's manual Start/Stop.
    fn editor_key(&self) -> String {
        self.editing_name
            .clone()
            .unwrap_or_else(|| self.profile.name.clone())
    }

    pub(crate) fn editor_running(&self) -> bool {
        self.engines
            .get(&self.editor_key())
            .map(|e| e.is_running())
            .unwrap_or(false)
    }

    fn push_modal(&mut self, modal: Modal) {
        self.modal_alpha = 0.0;
        self.modal.push(modal);
    }

    fn top_modal(&self) -> Option<Modal> {
        self.modal.last().copied()
    }

    fn sync_interval(&mut self) {
        let raw = self.interval_value.trim().replace(',', ".");
        if raw.is_empty() {
            return;
        }
        match raw.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => {
                let ms = (v * self.interval_unit.mult() as f64).round() as u64;
                self.profile.interval_ms = ms.max(1);
            }
            _ => self.status = format!("Invalid interval: {}", self.interval_value),
        }
    }

    fn refresh_fixed_inputs(&mut self) {
        if let ClickTarget::Fixed { x, y } = self.profile.target {
            self.fixed_x = x.to_string();
            self.fixed_y = y.to_string();
        }
    }

    /// Loads a profile into the editor working copy. Returns false on failure
    /// (caller should stay on the current screen).
    fn load_into(&mut self, name: &str) -> bool {
        match load_profile(name) {
            Ok(p) => {
                let (value, unit) = split_interval(p.interval_ms);
                self.interval_value = value;
                self.interval_unit = unit;
                self.profile = p;
                self.editing_name = Some(name.to_string());
                self.refresh_fixed_inputs();
                self.status = format!("Loaded {name}");
                true
            }
            Err(e) => {
                self.status = format!("Load failed: {e}");
                false
            }
        }
    }

    fn engine_mut(&mut self, name: &str) -> &mut ClickEngine {
        self.engines.entry(name.to_string()).or_default()
    }

    fn start_engine(&mut self, name: &str, config: ClickConfig) {
        self.errors.remove(name);
        self.engine_mut(name).start(config);
        self.status = format!("Running ({name})");
    }

    fn stop_engine(&mut self, name: &str) {
        if let Some(engine) = self.engines.get_mut(name) {
            engine.stop();
        }
        if self.status.starts_with("Running") {
            self.status = "Stopped.".to_string();
        }
    }

    fn disarm(&mut self, name: &str) {
        if let Some(armed) = self.armed.remove(name) {
            if let EffectiveTrigger::Hotkey { key, .. } = armed.trigger {
                self.hotkeys.unregister_key(key);
            }
        }
        self.stop_engine(name);
    }

    fn toggle_arm(&mut self, name: &str) {
        if self.errors.remove(name).is_some() {
            self.disarm(name);
            self.status = format!("Cleared error on {name}.");
            return;
        }
        if self.armed.contains_key(name) {
            self.disarm(name);
            self.status = "Stopped.".to_string();
            return;
        }
        let p = match load_profile(name) {
            Ok(p) => p,
            Err(e) => {
                self.status = format!("Load failed: {e}");
                return;
            }
        };
        let config = match profile_config(&p) {
            Ok(c) => c,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        let trigger = match self.resolve_effective_trigger(&p) {
            Ok(t) => t,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let press_mode = p.press_mode;
        let label = trigger_label(&p);
        self.stop_engine(name);
        self.armed.insert(
            name.to_string(),
            ArmedProfile {
                config,
                trigger,
                press_mode,
            },
        );
        self.status = format!("Listening — press {label} ({})", press_mode.label());
    }

    /// Resolve a profile's effective trigger against the mouse monitor, the
    /// armed set and the hotkey backend. The returned `Err` strings are the
    /// same user-visible reasons the old inline arming code produced.
    fn resolve_effective_trigger(&mut self, p: &Profile) -> Result<EffectiveTrigger, String> {
        if let Some(button) = p.mouse_trigger {
            if let MonitorStatus::Unavailable(reason) = self.mouse.status() {
                return Err(format!("Mouse triggers unavailable: {reason}"));
            }
            if self.armed.values().any(|a| {
                matches!(a.trigger, EffectiveTrigger::Mouse { button: b } if b == button)
            }) {
                return Err(format!("{button} is already armed by another profile."));
            }
            return Ok(EffectiveTrigger::Mouse { button });
        }
        let hotkey = parse_hotkey(&p.trigger_hotkey).map_err(|e| format!("Hotkey error: {e}"))?;
        if self.armed.values().any(|a| {
            matches!(a.trigger, EffectiveTrigger::Hotkey { id: hid, .. } if hid == hotkey.id())
        }) {
            return Err("This hotkey is already armed by another profile.".to_string());
        }
        self.hotkeys
            .register_hotkey(hotkey)
            .map_err(|e| format!("Hotkey error: {e}"))?;
        Ok(EffectiveTrigger::Hotkey {
            id: hotkey.id(),
            key: hotkey,
        })
    }

    /// Live resync of the armed/running state after a successful save:
    /// a matching snapshot is left untouched; anything else is rebuilt —
    /// the trigger is re-registered (possibly a new one), a running engine
    /// restarts with the new config, and a rename carries the state to the
    /// new key. A failed rebuild falls back to Idle with the reason shown.
    fn resync_after_save(&mut self, old: &str, new: &str) {
        let Some(armed) = self.armed.get(old).cloned() else {
            if old != new {
                self.engines.remove(old);
                self.errors.remove(old);
            }
            self.status = format!("Saved {new}");
            return;
        };
        if old == new && armed_matches_profile(&armed, &self.profile) {
            self.status = format!("Saved {new}");
            return;
        }
        let was_running = self
            .engines
            .get(old)
            .map(|engine| engine.is_running())
            .unwrap_or(false);
        self.armed.remove(old);
        if let EffectiveTrigger::Hotkey { key, .. } = &armed.trigger {
            self.hotkeys.unregister_key(key.clone());
        }
        self.engines.remove(old);
        self.errors.remove(old);

        let profile = self.profile.clone();
        let outcome = match self.resolve_effective_trigger(&profile) {
            Ok(trigger) => profile_config(&profile)
                .map(|config| (trigger, config))
                .map_err(|e| e.to_string()),
            Err(reason) => Err(reason),
        };
        match outcome {
            Ok((trigger, config)) => {
                self.armed.insert(
                    new.to_string(),
                    ArmedProfile {
                        config: config.clone(),
                        trigger,
                        press_mode: profile.press_mode,
                    },
                );
                if was_running {
                    self.start_engine(new, config);
                }
                self.status = format!("Saved {new} — armed state updated.");
            }
            Err(reason) => {
                self.engines.remove(new);
                self.status = format!("Saved {new}. Disarmed: {reason}");
            }
        }
    }

    /// Fires a trigger edge for one profile: `pressed=true` on press,
    /// `pressed=false` on release.
    fn fire(&mut self, name: &str, pressed: bool) {
        let armed = match self.armed.get(name).cloned() {
            Some(a) => a,
            None => return,
        };
        match (armed.press_mode, pressed) {
            (PressMode::Toggle, true) => {
                let running = self
                    .engines
                    .get(name)
                    .map(|e| e.is_running())
                    .unwrap_or(false);
                if running {
                    self.stop_engine(name);
                    self.status = "Stopped.".to_string();
                } else {
                    self.start_engine(name, armed.config.clone());
                }
            }
            (PressMode::Hold, true) => {
                let running = self
                    .engines
                    .get(name)
                    .map(|e| e.is_running())
                    .unwrap_or(false);
                if !running {
                    self.start_engine(name, armed.config.clone());
                }
            }
            (PressMode::Hold, false) => {
                let running = self
                    .engines
                    .get(name)
                    .map(|e| e.is_running())
                    .unwrap_or(false);
                if running {
                    self.stop_engine(name);
                    self.status = "Stopped.".to_string();
                }
            }
            _ => {}
        }
    }

    fn route_hotkey(&mut self, id: u32, pressed: bool) {
        let names: Vec<String> = self
            .armed
            .iter()
            .filter(|(_, a)| {
                matches!(a.trigger, EffectiveTrigger::Hotkey { id: hid, .. } if hid == id)
            })            .map(|(n, _)| n.clone())
            .collect();
        for name in names {
            self.fire(&name, pressed);
        }
    }

    fn route_mouse(&mut self, button: MouseTrigger, pressed: bool) {
        let names: Vec<String> = self
            .armed
            .iter()
            .filter(|(_, a)| {
                matches!(a.trigger, EffectiveTrigger::Mouse { button: b } if b == button)
            })
            .map(|(n, _)| n.clone())
            .collect();
        for name in names {
            self.fire(&name, pressed);
        }
    }

    /// Surfaces worker-thread backend failures (e.g. enigo panics caught in
    /// the engine) as per-profile ERROR states.
    fn drain_engine_errors(&mut self) {
        let mut failed = Vec::new();
        for (name, engine) in self.engines.iter() {
            if let Some(err) = engine.take_error() {
                failed.push((name.clone(), err));
            }
        }
        for (name, err) in failed {
            self.errors.insert(name.clone(), err.clone());
            self.status = format!("{name}: {err}");
        }
    }

    fn record_capture(&mut self, capture: Capture) {
        match capture {
            Capture::Hotkey(s) => {
                self.profile.trigger_hotkey = s.clone();
                self.profile.mouse_trigger = None;
                self.status = format!("Trigger captured: {s}");
                self.modal.pop();
            }
            Capture::Mouse(m) => {
                self.profile.mouse_trigger = Some(m);
                self.status = format!("Trigger captured: {m}");
                self.modal.pop();
            }
            Capture::Cancel => {
                self.status = "Capture cancelled.".to_string();
                self.modal.pop();
            }
            Capture::Unsupported => {
                self.status =
                    "That input can't be used — try another key or button (Esc cancels)."
                        .to_string();
            }
        }
    }
}

fn update(state: &mut RsClicker, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            if state.screen == Screen::Splash {
                state.splash_tick += 1;
                if state.splash_tick >= SPLASH_TICKS {
                    state.screen = Screen::Main;
                }
            }
            if !state.modal.is_empty() && state.modal_alpha < 0.55 {
                state.modal_alpha = (state.modal_alpha + 0.11).min(0.55);
            }
            for (id, hk_state) in state.hotkeys.poll_events() {
                state.route_hotkey(id, hk_state == HotKeyState::Pressed);
            }
            // Injected click pairs are dropped wholesale before routing or
            // capture so a profile's own clicks can never re-trigger it, and
            // a running engine cannot pollute a concurrent capture session.
            let mut batch = Vec::new();
            while let Ok(ev) = state.mouse.rx.try_recv() {
                batch.push(ev);
            }
            for ev in filter_injected_pairs(batch) {
                match ev {
                    MouseEvent::Press(button, _) => {
                        if state.top_modal() == Some(Modal::Capture) {
                            state.record_capture(Capture::Mouse(button));
                        } else {
                            state.route_mouse(button, true);
                        }
                    }
                    MouseEvent::Release(button, _) => {
                        if state.top_modal() != Some(Modal::Capture) {
                            state.route_mouse(button, false);
                        }
                    }
                }
            }
            state.drain_engine_errors();
        }
        Message::SkipSplash => {
            state.screen = Screen::Main;
        }
        Message::KeyEvent(event) => {
            if state.top_modal() == Some(Modal::Capture) {
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                    state.record_capture(capture_hotkey(&key, &modifiers));
                }
            }
        }
        Message::OpenNewProfile => {
            state.new_profile_name.clear();
            state.dialog_error.clear();
            state.push_modal(Modal::NewProfile);
        }
        Message::NewProfileNameChanged(v) => {
            state.new_profile_name = v;
            state.dialog_error.clear();
        }
        Message::ConfirmNewProfile => {
            let name = state.new_profile_name.trim().to_string();
            if name.is_empty() {
                state.dialog_error = "Please enter a profile name.".to_string();
            } else if profile_file_exists(&name) {
                state.dialog_error = "A profile with this name already exists.".to_string();
            } else {
                let profile = Profile::new(&name);
                match save_profile(&profile) {
                    Ok(()) => {
                        state.profiles = list_profiles();
                        state.modal.pop();
                    }
                    Err(e) => state.dialog_error = format!("Save failed: {e}"),
                }
            }
        }
        Message::CloseModal => {
            let popped = state.modal.pop();
            if popped == Some(Modal::Editor) {
                state.profiles = list_profiles();
            }
        }
        Message::OpenEditor(name) => {
            if state.load_into(&name) {
                state.push_modal(Modal::Editor);
            }
        }
        Message::DeleteProfile => {
            if let Some(name) = state.editing_name.clone() {
                state.disarm(&name);
                state.engines.remove(&name);
                state.errors.remove(&name);
                match delete_profile(&name) {
                    Ok(()) => state.status = format!("Deleted {name}."),
                    Err(e) => state.status = format!("Delete failed: {e}"),
                }
                state.modal.retain(|m| *m != Modal::Editor);
                state.profiles = list_profiles();
            }
        }
        Message::OpenAppSettings => {
            state.pending_settings = state.settings;
            state.push_modal(Modal::AppSettings);
        }
        Message::ApplySettings => {
            state.settings = state.pending_settings;
            if let Err(e) = save_settings(&state.settings) {
                state.status = format!("Settings save failed: {e}");
            }
            state.modal.pop();
        }
        Message::SetTheme(base) => state.pending_settings.base = base,
        Message::SetAccent(accent) => state.pending_settings.accent = accent,
        Message::SetViewMode(view) => state.pending_settings.view = view,
        Message::OpenAbout => state.push_modal(Modal::About),
        Message::OpenLicense => state.push_modal(Modal::License),
        Message::OpenCapture => state.push_modal(Modal::Capture),
        Message::ToggleArm(name) => state.toggle_arm(&name),
        Message::IntervalValueChanged(v) => {
            state.interval_value = v;
            state.sync_interval();
        }
        Message::IntervalUnitSelected(u) => {
            state.interval_unit = u;
            state.sync_interval();
        }
        Message::HotkeyChanged(v) => {
            // Manual keyboard entry takes over from any mouse trigger.
            state.profile.mouse_trigger = None;
            state.profile.trigger_hotkey = v;
        }
        Message::PressModeSelected(m) => state.profile.press_mode = m,
        Message::ButtonSelected(b) => state.profile.button = b,
        Message::KindSelected(k) => state.profile.kind = k,
        Message::TargetCurrent => state.profile.target = ClickTarget::CurrentPosition,
        Message::TargetFixed => {
            let x = state.fixed_x.parse().unwrap_or(500);
            let y = state.fixed_y.parse().unwrap_or(300);
            state.profile.target = ClickTarget::Fixed { x, y };
        }
        Message::TargetSequence => {
            state.profile.target = ClickTarget::Sequence(Vec::new());
        }
        Message::FixedXChanged(v) => {
            match v.trim().parse::<i32>() {
                Ok(x) => {
                    state.fixed_x = v;
                    if let ClickTarget::Fixed { x: _, y } = state.profile.target {
                        state.profile.target = ClickTarget::Fixed { x, y };
                    }
                }
                Err(_) if !v.trim().is_empty() => {
                    state.status = format!("Invalid X coordinate: {v}");
                }
                _ => state.fixed_x = v,
            }
        }
        Message::FixedYChanged(v) => {
            match v.trim().parse::<i32>() {
                Ok(y) => {
                    state.fixed_y = v;
                    if let ClickTarget::Fixed { x, y: _ } = state.profile.target {
                        state.profile.target = ClickTarget::Fixed { x, y };
                    }
                }
                Err(_) if !v.trim().is_empty() => {
                    state.status = format!("Invalid Y coordinate: {v}");
                }
                _ => state.fixed_y = v,
            }
        }
        Message::PointXChanged(v) => state.point_x = v,
        Message::PointYChanged(v) => state.point_y = v,
        Message::AddPoint => {
            if let (Ok(x), Ok(y)) = (state.point_x.parse(), state.point_y.parse()) {
                state.profile.points.push(ClickPoint { x, y });
                state.status = format!("Point added ({} total)", state.profile.points.len());
            } else {
                state.status = "Invalid point coordinates".to_string();
            }
        }
        Message::ClearPoints => {
            state.profile.points.clear();
            state.status = "Points cleared".to_string();
        }
        Message::ProfileNameChanged(v) => state.profile.name = v,
        Message::SaveProfile => {
            let new_name = state.profile.name.trim().to_string();
            if new_name.is_empty() {
                state.status = "Profile name can't be empty.".to_string();
            } else {
                let old = state
                    .editing_name
                    .clone()
                    .unwrap_or_else(|| new_name.clone());
                state.profile.name = new_name.clone();
                if new_name != old
                    && profile_file_exists(&new_name)
                    && !same_profile_file(&old, &new_name)
                {
                    state.status = "Another profile already uses that name.".to_string();
                    state.profile.name = old;
                } else {
                    match save_profile(&state.profile) {
                        Ok(()) => {
                            if new_name != old {
                                // Renaming orphans the old file; live state
                                // is carried over by the resync step.
                                let _ = delete_profile(&old);
                                state.editing_name = Some(new_name.clone());
                            }
                            state.resync_after_save(&old, &new_name);
                            state.profiles = list_profiles();
                        }
                        Err(e) => state.status = format!("Save failed: {e}"),
                    }
                }
            }
        }
        Message::Start => {
            let key = state.editor_key();
            match profile_config(&state.profile) {
                Ok(config) => state.start_engine(&key, config),
                Err(e) => state.status = e.to_string(),
            }
        }
        Message::Stop => {
            let key = state.editor_key();
            state.stop_engine(&key);
            state.status = "Stopped.".to_string();
        }
    }
    Task::none()
}

fn subscription(_: &RsClicker) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick),
        keyboard::listen().map(Message::KeyEvent),
    ])
}

fn view(state: &RsClicker) -> Element<'_, Message> {
    let base: Element<'_, Message> = match state.screen {
        Screen::Splash => views::splash::splash_view(state),
        Screen::Main => views::main::main_view(state),
    };
    match state.modal.last() {
        None => base,
        Some(Modal::NewProfile) => stack([
            base,
            opaque(views::backdrop(state.modal_alpha, views::dialogs::new_profile_dialog(state))),
        ])
        .into(),
        Some(Modal::Editor) => stack([
            base,
            opaque(views::backdrop(
                state.modal_alpha,
                views::dialog_card(views::editor::editor_card(state), 400.0),
            )),
        ])
        .into(),
        Some(Modal::Capture) => stack([
            base,
            opaque(views::backdrop(
                state.modal_alpha,
                views::dialogs::capture_dialog(state),
            )),
        ])
        .into(),
        Some(Modal::AppSettings) => stack([
            base,
            opaque(views::backdrop(
                state.modal_alpha,
                views::dialogs::app_settings_dialog(state),
            )),
        ])
        .into(),
        Some(Modal::About) => stack([
            base,
            opaque(views::backdrop(
                state.modal_alpha,
                views::dialogs::about_dialog(),
            )),
        ])
        .into(),
        Some(Modal::License) => stack([
            base,
            opaque(views::backdrop(
                state.modal_alpha,
                views::dialogs::license_dialog(),
            )),
        ])
        .into(),
    }
}
