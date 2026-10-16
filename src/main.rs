mod app;
mod atomic_file;
mod clicker;
mod hotkey;
mod mouse;
mod profile;
mod settings_store;
mod theme;
mod trigger;
mod views;

fn main() -> iced::Result {
    // Worker threads (click engine, evdev monitor) report failures through
    // error flags/channels, so their panic messages would only scare users
    // (notably enigo's internal RemoteDesktop unwrap on portal-less systems).
    // Silence those; everything else still prints normally.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let silenced = std::thread::current()
            .name()
            .map(|n| n.starts_with("rsclicker-"))
            .unwrap_or(false);
        if !silenced {
            default_hook(info);
        }
    }));

    // On Linux, default to the tiny-skia (software) renderer unless the user
    // overrides it. Reason: on old Intel GPUs (e.g. Haswell HD 4400) wgpu
    // tries Vulkan first, whose support is incomplete ("MESA-INTEL: warning:
    // Haswell Vulkan support is incomplete"), so the window never appears.
    // tiny-skia is more than fast enough for this simple UI.
    // Override with e.g. ICED_BACKEND=wgpu.
    #[cfg(target_os = "linux")]
    if std::env::var_os("ICED_BACKEND").is_none() {
        std::env::set_var("ICED_BACKEND", "tiny-skia");
    }
    app::run()
}
