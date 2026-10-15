//! `build.rs`からの埋め込みが実際に効くことを、EXEをビルドして確かめる(MSVCのWindowsのみ)。
#![cfg(all(windows, target_env = "msvc"))]

use rs_dpi_awareness::{read_embedded_manifest, Awareness, Declared};
use std::path::PathBuf;
use std::process::Command;

fn build_fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target =
        std::env::temp_dir().join(format!("rs-dpiawareness-fixture-{}", std::process::id()));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(root.join("tests/fixtures/manifest-app/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("cargo build");
    assert!(
        out.status.success(),
        "fixtureのビルドに失敗: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    target.join("debug").join("manifest-app.exe")
}

#[test]
fn an_embedded_manifest_alone_makes_the_process_per_monitor_v2() {
    let exe = build_fixture();
    let exe_s = exe.to_string_lossy().to_string();

    // 1) 埋め込まれたマニフェストを読み出せて、PerMonitorV2を宣言している。
    let xml = read_embedded_manifest(&exe_s)
        .expect("read")
        .expect("マニフェストが埋め込まれているはず");
    let d = Declared::parse(&xml);
    assert_eq!(d.effective(), Some(Awareness::PerMonitorV2), "{xml}");
    assert_eq!(d.legacy(), Some(Awareness::PerMonitor));

    // 2) 実際に起動すると、init()を呼ばなくてもPerMonitorV2で動いている(=マニフェストをWindowsが採用した)。
    let run = Command::new(&exe).output().expect("run fixture");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(stdout.trim(), "awareness=PerMonitorV2", "{stdout}");

    // 3) CLIの`check`が同じ結論を出す。
    let cli = Command::new(env!("CARGO_BIN_EXE_rs-dpiawareness"))
        .args(["check", &exe_s])
        .output()
        .expect("cli");
    let text = String::from_utf8_lossy(&cli.stdout);
    assert!(
        cli.status.success() && text.contains("PerMonitorV2"),
        "{text}"
    );

    let _ = std::fs::remove_dir_all(exe.parent().unwrap().parent().unwrap());
}

#[test]
fn the_cli_reports_a_missing_manifest_honestly() {
    // このCLI自身のEXEにはマニフェストを埋め込んでいない(実行時にinit()を呼ぶ作り)。
    let cli_exe = env!("CARGO_BIN_EXE_rs-dpiawareness");
    assert_eq!(read_embedded_manifest(cli_exe).expect("read"), None);
    let out = Command::new(cli_exe)
        .args(["check", cli_exe])
        .output()
        .expect("cli");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("埋め込みなし"),
        "{text}"
    );
}

#[test]
fn checking_a_missing_file_fails_with_a_clear_message() {
    let out = Command::new(env!("CARGO_BIN_EXE_rs-dpiawareness"))
        .args(["check", r"C:\definitely\not\here.exe"])
        .output()
        .expect("cli");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("見つかりません"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
