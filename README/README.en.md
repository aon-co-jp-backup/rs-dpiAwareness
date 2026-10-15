# rs-dpiAwareness

**Languages**: [日本語](../README.md) | English

A small, dependency-free Rust crate and inspector for Windows DPI awareness (**PerMonitorV2**).
It is a Rust tribute to Microsoft's public specification (`SetProcessDpiAwarenessContext` and the `<dpiAware>` / `<dpiAwareness>` elements of the application manifest),
written from the public documentation only and not affiliated with Microsoft.

## Why

An app that does not declare DPI awareness is **bitmap-stretched by Windows on high-DPI screens and looks blurry**.
With `PerMonitorV2` (Windows 10 1703+) it follows each monitor's DPI, and title bars, dialogs and common controls scale automatically.
Several things are easy to get wrong, and this crate fills those gaps:

- **It cannot be changed once set.** A process's DPI awareness must be chosen before any window is created.
- **Static linking breaks old Windows.** Linking `SetProcessDpiAwarenessContext` and friends statically stops the EXE from starting on Windows versions that lack them. Here they are resolved at run time.
- **A manifest is the most reliable.** A declaration is fixed before the process starts, so nothing slips through. `build.rs` can embed it in one line.
- **"Did it work?" is hard to tell.** The value is **read back and reported**, and you can inspect what an existing EXE declares.

## The simplest use

```toml
# Cargo.toml (not on crates.io yet; use a Git dependency for now)
[dependencies]
rs-dpi-awareness = { git = "https://github.com/aon-co-jp/rs-dpiAwareness" }
```

```rust
fn main() {
    // Once, before creating any window. Falls back PerMonitorV2 -> PerMonitor -> System on older Windows; safe to call repeatedly.
    let outcome = rs_dpi_awareness::init();
    println!("{outcome}");
    // ...create windows...
}
```

### Declare it in the manifest (recommended)

```toml
[build-dependencies]
rs-dpi-awareness = { git = "https://github.com/aon-co-jp/rs-dpiAwareness" }
```

```rust
// build.rs
fn main() {
    rs_dpi_awareness::build::embed(rs_dpi_awareness::Awareness::PerMonitorV2);
}
```

This embeds the manifest in MSVC-target EXEs (nothing happens on other targets; `*-pc-windows-gnu` is unsupported and only prints a warning without failing the build).
For finer control (common controls v6, long paths, UTF-8 code page) use `Manifest::new(..).common_controls_v6(true)...` with `build::embed_manifest_or_warn(&m)`.

### Following `WM_DPICHANGED`

[`examples/window.rs`](../examples/window.rs) is a complete minimal Win32 example with no extra crates (`cargo run --example window`). The core is four lines:

```rust
// inside the window procedure
WM_DPICHANGED => {
    if let Some(d) = unsafe { DpiChanged::from_message(wparam, lparam) } {
        let r = d.suggested; // the new position and size Windows suggests
        unsafe { SetWindowPos(hwnd, 0, r.left, r.top, r.width(), r.height(), SWP_NOZORDER | SWP_NOACTIVATE) };
    }
    0
}
```

### Converting pixels

```rust
use rs_dpi_awareness::scale;
assert_eq!(scale::px(100, 144), 150);   // 100 logical px (96 dpi) -> 150 physical px at 144 dpi (150%)
assert_eq!(scale::unpx(150, 144), 100); // the reverse
assert_eq!(scale::percent(168), 175);
```

Rounding is the same as Windows' `MulDiv` (to nearest, halves away from zero), so results match Windows' own conversions to the pixel.

## The inspector `rs-dpiawareness`

```
rs-dpiawareness info                  show this PC's DPI awareness and monitors
rs-dpiawareness check <exe>           check whether an EXE's manifest declares DPI awareness (does not run it)
rs-dpiawareness manifest [--awareness v2|pm|system|unaware|gdi] [--common-controls] [--long-path] [--utf8]
rs-dpiawareness scale <px> <dpi>      logical px -> physical px      (unscale: the reverse)
```

Example (this PC, 4K at 200%):

```
DPI awareness: PerMonitorV2 (requested PerMonitorV2, applied via ContextApi)
system DPI: 192 (200%)
monitor 1 \\.\DISPLAY1 [primary]: 3840x2160 @(0,0)  DPI 192x192 = 200%  work area 3840x2064
```

`check` reads the embedded manifest's `<dpiAwareness>` / `<dpiAware>` and shows both the level **Windows 10 1607+ actually uses** and the level older Windows uses.

## What was verified / what was not (honest disclosure)

Verified (Windows 11 Home 10.0.26200, MSVC, one 4K monitor at 200%):

- `init()` turns a manifest-less process into `PerMonitorV2` and reads the value back. A second call reports `AlreadySet`, and asking for a weaker level does not lower it.
- With **only an embedded manifest** (no `init()` at run time), a freshly built EXE really runs as `PerMonitorV2`; the manifest read back from the EXE matches.
- A real window follows a synthetic `WM_DPICHANGED` (216 dpi, with a suggested rectangle): size and title change (`--selftest` of `examples/window.rs`).
- Monitor count, position, work area and DPI; at 200% the window frame and scroll-bar width roughly double.

**Not verified**:

- **Moving a window between monitors with different DPIs** was not tried on real hardware (only one monitor here). `WM_DPICHANGED` is checked with a synthetic message.
- The fallbacks (`PerMonitor` / `System`) on **Windows older than 10 1703** are implemented but not tried on a real old OS.
- Manifest embedding for `*-pc-windows-gnu` is unsupported (run-time `init()` works).
- DPI awareness is **per process**. If a thread overrides it with `SetThreadDpiAwarenessContext`, `process_awareness()` returns that thread's value.
- Frameworks such as winit, Tauri and Qt set DPI awareness themselves; with them, prefer the framework's setting (this crate suits hand-written Win32/GDI apps and diagnostics).
- On non-Windows targets everything is a harmless no-op (`monitors()` is empty, `system_dpi()` is 96).
- Not published on crates.io yet.

## Related

- [`dpi` from winit](https://crates.io/crates/dpi) distinguishes logical and physical sizes with **types**; it complements rather than replaces this crate.
- [RPoem](https://github.com/aon-co-jp/RPoem) is not used: this is a low-level piece that calls OS APIs directly, with no room for a web framework.
- Sibling Rust tributes: [rs-FFmpeg](https://github.com/aon-co-jp/rs-FFmpeg) / [rs-xorriso](https://github.com/aon-co-jp/rs-xorriso)

License: MIT
