//! 実行時に`init()`を呼ばない。マニフェストだけでPerMonitorV2になることを確かめるための最小のアプリ。
fn main() {
    println!("awareness={}", rs_dpi_awareness::process_awareness());
}
