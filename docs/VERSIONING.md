# Versioning

RsClicker uses **date-based releases**: `YY.MM.DD` — two-digit year, month,
day of the release build. Examples:

- `26.10.15` — released 15 October 2026.
- `26.10.19` — current release, built 19 October 2026.

The Cargo package version intentionally stays at `0.1.0` (stable crate
metadata); the release version is recorded separately:

```toml
# Cargo.toml
[package.metadata.rsclicker]
release = "26.10.19"
```

and mirrored in `src/app.rs` (`const VERSION`), which is what the splash
screen, main-screen header and About dialog display. When cutting a new
release, bump **both** places and tag the commit accordingly.
