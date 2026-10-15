# Development policy & environment rules (rs-dpiAwareness)

Rules shared by all repositories (autonomous continuation, verification on real machines, UI / installer quality rules, etc.)
are canonical in [`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)'s `CLAUDE.md`. This document only covers what is specific to this repository.

## Role

A dependency-free Rust crate for Windows DPI awareness (PerMonitorV2) plus an inspector CLI (`rs-dpiawareness`).
Like `rs-FFmpeg` and `rs-xorriso`, it is a "Rust tribute to a public specification" and is not affiliated with Microsoft.
Write code from public documentation only (do not copy code from Microsoft, winit, etc.).

## Layout

- `src/awareness.rs` — `Awareness`, `init()` / `set()` / `process_awareness()`, graceful fallback, read-back of the result
- `src/sys.rs` — Win32 layer. **Newer APIs are resolved with `GetProcAddress` at run time** (static linking would stop the EXE from starting on old Windows)
- `src/monitor.rs` — monitor enumeration, DPI, `WM_DPICHANGED` (`DpiChanged`), per-DPI window sizes
- `src/scale.rs` — logical <-> physical pixel conversion (same rounding as `MulDiv`)
- `src/manifest.rs` — manifest generation (`Manifest`), parsing (`Declared`), reading it from an EXE
- `src/build.rs` — embedding helper for `build.rs` (MSVC only; uses the generic `cargo:rustc-link-arg` because kind-specific `-bins` etc. make Cargo error out in packages lacking that kind)
- `src/bin/rs-dpiawareness.rs` — inspector CLI
- `examples/window.rs` — a real-window example; `--selftest` checks that it follows a synthetic `WM_DPICHANGED`
- `tests/` — real-machine tests. **`windows_apply.rs` has exactly one test** (DPI awareness cannot change once set, so several tests in one process would depend on run order). `embed_manifest.rs` really builds an EXE.

## Rules

- Do not add dependencies (zero dependencies is the selling point).
- When adding a Windows API, first consider resolving it with `GetProcAddress`.
- Keep the "verified / not verified" section of the README honest. Moving between monitors, Windows older than 10 1703 and GNU targets are unverified.
- Verify on a real Windows machine; do not call it done on CI success alone.
- Before committing: `cargo fmt` / `cargo clippy --all-targets -- -D warnings` / `cargo test -- --test-threads=1`.
