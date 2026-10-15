//! 実機(Windows)での「設定する前後」のテスト。**このファイルのテストは1つだけ**にしている:
//! テストは同じプロセス内で動き、DPI認識は一度決まると変えられないため、複数あると実行順で結果が変わってしまう。
#![cfg(windows)]

use rs_dpi_awareness::{init, process_awareness, set, Awareness, Method, Source};

#[test]
fn init_makes_this_process_per_monitor_v2_and_is_idempotent() {
    // このテストのEXEにはマニフェストが無いので、最初はDPI非対応のはず。
    assert_eq!(
        process_awareness(),
        Awareness::Unaware,
        "テスト用EXEは宣言なし=非対応で始まるはず"
    );

    let first = init();
    eprintln!("{first}");
    assert_eq!(
        first.achieved,
        Awareness::PerMonitorV2,
        "Windows 10 1703以降ならPerMonitorV2になる: {first:?}"
    );
    assert_eq!(first.source, Source::Applied);
    assert_eq!(first.method, Method::ContextApi);
    assert!(first.is_satisfied());
    assert_eq!(
        process_awareness(),
        Awareness::PerMonitorV2,
        "設定後に取得し直しても同じ"
    );

    // 2回目: 既に決まっているので何も変えず、そう報告する。
    let second = init();
    assert_eq!(second.achieved, Awareness::PerMonitorV2);
    assert_eq!(second.source, Source::AlreadySet);
    assert_eq!(second.method, Method::None);

    // 弱いレベルを頼んでも、既に強いので下げない(Windowsは変更を許さない)。
    let weaker = set(Awareness::System);
    assert_eq!(weaker.achieved, Awareness::PerMonitorV2);
    assert_eq!(weaker.source, Source::AlreadySet);
    assert!(weaker.is_satisfied());

    // 同じ強さ以下を何度頼んでも、落ちずに同じ結果になる。
    assert_eq!(set(Awareness::PerMonitor).achieved, Awareness::PerMonitorV2);
    assert_eq!(set(Awareness::Unaware).achieved, Awareness::PerMonitorV2);
}
