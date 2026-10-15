//! 実物のウィンドウで`WM_DPICHANGED`の扱いを確かめる(サンプル`examples/window.rs`の自己診断を、そのまま実行する)。
#![cfg(windows)]

#[path = "../examples/window.rs"]
#[allow(dead_code)]
mod window_example;

#[test]
fn a_real_window_follows_a_synthetic_wm_dpichanged() {
    window_example::selftest().expect("selftest");
}
