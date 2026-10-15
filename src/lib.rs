//! # rs-dpi-awareness
//!
//! Windowsの**DPI認識(DPI awareness)**、とくに最新の`PerMonitorV2`を、依存ゼロのRustで扱う小さなクレート。
//! Microsoftの公開仕様(`SetProcessDpiAwarenessContext`、アプリケーションマニフェストの`<dpiAwareness>`)への
//! 敬意を込めたRust版で、Microsoftとは無関係(コードは公開ドキュメントだけを根拠に書いた)。
//!
//! A tiny, dependency-free Rust tribute to Windows **DPI awareness** (especially `PerMonitorV2`).
//! It is written from the public documentation only and is not affiliated with Microsoft.
//!
//! ## 一番かんたんな使い方 / The simplest use
//!
//! ```no_run
//! # #[allow(clippy::needless_doctest_main)]
//! fn main() {
//!     // ウィンドウを作る前に1回だけ。古いWindowsでも段階的に代替し、何度呼んでも安全。
//!     let outcome = rs_dpi_awareness::init();
//!     println!("{outcome}");
//!     // ...ウィンドウ作成...
//! }
//! ```
//!
//! ## 何ができるか / What it offers
//!
//! | 機能 | 入口 |
//! |---|---|
//! | 認識レベルの設定(`PerMonitorV2 → PerMonitor → System`と段階的に代替し、結果を再取得して確認) | [`init`] / [`set`] → [`Outcome`] |
//! | 今のプロセスの認識レベルの取得 | [`process_awareness`] |
//! | モニターごとのDPI・位置・作業領域 | [`monitors`] / [`system_dpi`] / [`window_dpi`] |
//! | `WM_DPICHANGED`の解釈(提案される新しい矩形つき) | [`DpiChanged`] |
//! | 論理⇔物理ピクセル変換(Windowsの`MulDiv`と同じ丸め) | [`scale`] |
//! | DPIを考慮したウィンドウ寸法・システム寸法 | [`window_size_for_client`] / [`system_metric`] |
//! | アプリケーションマニフェストの生成・解析・EXEからの読み出し | [`Manifest`] / [`Declared`] / [`read_embedded_manifest`] |
//! | `build.rs`から1行でマニフェストをEXEへ埋め込む(MSVC) | [`build`] |
//!
//! Windows以外では、すべて**何もしない安全な値**を返す(クロスプラットフォームのアプリが無条件に呼べる)。
//! On non-Windows targets everything is a harmless no-op, so cross-platform apps can call it unconditionally.

mod awareness;
pub mod build;
mod manifest;
mod monitor;
pub mod scale;
#[cfg(windows)]
mod sys;

pub use awareness::{init, process_awareness, set, Awareness, Method, Outcome, Source};
pub use manifest::{read_embedded_manifest, Declared, Manifest, ManifestError};
pub use monitor::{
    monitors, system_dpi, system_metric, window_dpi, window_size_for_client, DpiChanged,
    MonitorInfo, Rect, BASE_DPI, WM_DPICHANGED,
};

/// このビルドでDPI認識の設定ができるか(Windowsのみ`true`)。
pub const fn is_supported() -> bool {
    cfg!(windows)
}
