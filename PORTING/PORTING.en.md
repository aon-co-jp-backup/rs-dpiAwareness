# PORTING (rs-dpiAwareness)

日本語: [PORTING.md](../PORTING.md)

## Status (2026-10-16, v0.1.0)

Implemented and verified on a real machine (Windows 11, MSVC, one 4K monitor at 200%):
- `init()` / `set()` / `process_awareness()` (graceful fallback, read-back, idempotent), monitor enumeration, `WM_DPICHANGED`, per-DPI window sizes, conversions,
  manifest generation / parsing / reading from an EXE, embedding from `build.rs` (an EXE with only the embedded manifest really runs as PerMonitorV2), the inspector CLI and a real-window example.
- Tests: 26 unit tests plus real-machine / integration tests (`windows_apply`, `windows_runtime`, `windows_window`, `embed_manifest`).

## Not done / next

1. Real-hardware check of **moving between monitors with different DPIs** (only a synthetic `WM_DPICHANGED` so far).
2. Real-OS check of the fallbacks (PerMonitor / System) on **Windows older than 10 1703**.
3. Manifest embedding for `*-pc-windows-gnu` (via `windres`); today it only warns.
4. A small pure-Rust PE reader so `check` works off Windows too.
5. Publish to crates.io (after confirming the name `rs-dpi-awareness` is free).
6. Check the CI results (GitHub's Windows runners may have no display; if the monitor test fails, relax its condition).

## Design decisions (with reasons)

- **Resolve newer APIs at run time**: static linking stops the EXE from starting on old Windows.
- **Return the value read back**: never report "set" when it is not; if it was already fixed, say `AlreadySet`.
- **One test in `windows_apply.rs`**: DPI awareness cannot change once set, so several tests in one process would depend on run order.
- **Use the generic `cargo:rustc-link-arg`**: emitting `-bins` / `-examples` / `-tests` separately makes Cargo error out in packages lacking that kind (this actually happened while building the fixture).
- **No RPoem**: this is a low-level piece calling OS APIs directly; there is no place for a web framework.
