# Building

## Linux

Install the system linker dependency for the GUI stack:

```bash
# Void Linux
sudo xbps-install -S base-devel libxkbcommon-devel
# Debian / Ubuntu / Mint / Pop!_OS
sudo apt install build-essential pkg-config libxkbcommon-dev
# Arch / Manjaro / EndeavourOS
sudo pacman -S base-devel libxkbcommon
# Fedora / RHEL / Rocky / AlmaLinux
sudo dnf install gcc pkg-config libxkbcommon-devel
# openSUSE (Tumbleweed / Leap)
sudo zypper install gcc pkg-config libxkbcommon-devel
# Gentoo
sudo emerge -av libxkbcommon pkgconfig
# Alpine
sudo apk add build-base pkgconf libxkbcommon-dev
```

Then:

```bash
cargo build --release
./target/release/RsClicker
```

For global mouse triggers, your user must be able to read `/dev/input`:

```bash
sudo usermod -aG input "$USER"   # log out and back in afterwards
```

On old Intel GPUs (e.g. Haswell) the app defaults to software rendering
(`ICED_BACKEND=tiny-skia`); override with `ICED_BACKEND=wgpu` if you prefer.

## Windows 10/11 (cross-compile from Linux)

1. Install the MinGW cross toolchain and the Windows target:
   ```bash
   # Void Linux
   sudo xbps-install -S cross-x86_64-w64-mingw32
   # Debian / Ubuntu / Mint
   sudo apt install mingw-w64
   # Arch / Manjaro
   sudo pacman -S mingw-w64-gcc
   # Fedora / RHEL
   sudo dnf install mingw64-gcc
   # openSUSE
   sudo zypper install mingw64-cross-gcc
   # Any distro (via rustup, same everywhere)
   rustup target add x86_64-pc-windows-gnu
   ```
2. The repo already contains `.cargo/config.toml` pointing the
   `x86_64-pc-windows-gnu` target at `x86_64-w64-mingw32-gcc`.
3. Build and type-check:
   ```bash
   cargo check --target x86_64-pc-windows-gnu   # fast, no linking
   cargo build --release --target x86_64-pc-windows-gnu
   ```
4. The binary lands at `target/x86_64-pc-windows-gnu/release/RsClicker.exe`
   — copy it to a Windows 10/11 machine and double-click. No installer or
   extra setup needed (hotkeys, mouse hooks and clicking are all native).

## App icon (logo comes later)

The unique logo will be added in a later release. When it exists:

- **Linux:** export `assets/icon.png` (256×256) + `assets/icon.svg`, and
  ship `assets/RsClicker.desktop`:
  ```ini
  [Desktop Entry]
  Type=Application
  Name=RsClicker
  Exec=/path/to/RsClicker
  Icon=/path/to/assets/icon.png
  Categories=Utility;
  ```
  Install with `desktop-file-install` or by copying to
  `~/.local/share/applications/`.
- **Windows:** convert the logo to `assets/icon.ico` and embed it at build
  time with the [`winres`](https://crates.io/crates/winres) crate via a
  `build.rs` (`if cfg!(windows) { WinRes::new().set_icon(...) }`).
