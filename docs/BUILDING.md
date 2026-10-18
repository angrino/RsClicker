# Building

## Prerequisites

Install the Rust toolchain with [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

<!-- TODO: state the minimum supported Rust version (MSRV) here -->

## Linux

Install the build dependencies for the GUI stack:

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

<!-- TODO: note which distros you actually tested; if a build fails for you, please open an issue -->

For global mouse triggers, your user must be able to read `/dev/input`:

```bash
sudo usermod -aG input "$USER"   # log out and back in afterwards
```

Optional — to use the kernel-level uinput injection backend (Settings →
Input emulation → uinput), your user also needs write access to
`/dev/uinput`:

```bash
sudo usermod -aG uinput "$USER"  # log out and back in afterwards
```

Then build and run:

```bash
cargo build --release
./target/release/RsClicker
```

On old Intel GPUs (e.g. Haswell) the app defaults to software rendering
(`ICED_BACKEND=tiny-skia`); override with `ICED_BACKEND=wgpu` if you prefer.

## Windows 10/11

### Option A: build natively on Windows (MSVC)

1. Install [Rust via rustup](https://rustup.rs) and the
   **Visual Studio Build Tools** ("Desktop development with C++").
2. Build:
   ```powershell
   cargo build --release
   ```
3. The binary is at `target\release\RsClicker.exe`.

### Option B: cross-compile from Linux (MinGW)

1. Install the MinGW cross toolchain:
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
   ```
2. Add the Rust target (all distros):
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```
3. The repo already contains `.cargo/config.toml` pointing the
   `x86_64-pc-windows-gnu` target at `x86_64-w64-mingw32-gcc`.
4. Build and type-check:
   ```bash
   cargo check --target x86_64-pc-windows-gnu   # fast, no linking
   cargo build --release --target x86_64-pc-windows-gnu
   ```
5. The binary lands at `target/x86_64-pc-windows-gnu/release/RsClicker.exe`.
   Copy it to a Windows 10/11 machine and double-click. No installer or
   extra setup needed.

### Antivirus / SmartScreen warnings

Auto clickers use global input hooks and `SendInput`, which some antivirus
products and Windows SmartScreen flag as suspicious. These are false positives
on unsigned binaries. You can verify the source and build it yourself using the
steps above.

## Release packaging

Verified release binaries are collected under a dated folder in the repo
(not committed to git — they ship via GitHub Releases):

```bash
mkdir -p releases/26.10.19
cp target/release/RsClicker                              releases/26.10.19/RsClicker-linux-x86_64
cp target/x86_64-pc-windows-gnu/release/RsClicker.exe    releases/26.10.19/RsClicker-windows-x64.exe
```

Tag the release so its tree stays findable:

```bash
git tag v26.10.19
git push origin v26.10.19
```

Before packaging, run the release gate (see `docs/FIXLOG.md` for each
session's verification): `cargo test --release`, then both builds above
must finish clean.
