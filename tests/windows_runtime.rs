//! 実機(Windows)でのDPI・モニター・ウィンドウ寸法のテスト(`init()`の後の値を確かめる)。
#![cfg(windows)]

use rs_dpi_awareness::{init, monitors, system_dpi, system_metric, window_size_for_client};

#[test]
fn dpi_queries_return_sane_values_after_init() {
    init();
    let sys = system_dpi();
    assert!((96..=960).contains(&sys), "システムDPIが不自然: {sys}");
    let ms = monitors();
    eprintln!("monitors: {ms:#?}");
    for m in &ms {
        assert!(
            (96..=960).contains(&m.dpi_x) && (96..=960).contains(&m.dpi_y),
            "{m:?}"
        );
        assert!(m.rect.width() > 0 && m.rect.height() > 0, "{m:?}");
        assert!(
            m.work_rect.width() <= m.rect.width() && m.work_rect.height() <= m.rect.height(),
            "{m:?}"
        );
        assert!(m.device_name.starts_with(r"\\.\"), "{m:?}");
    }
    if !ms.is_empty() {
        assert_eq!(
            ms.iter().filter(|m| m.primary).count(),
            1,
            "メインモニターはちょうど1枚"
        );
    }
}

#[test]
fn window_geometry_grows_with_dpi() {
    init();
    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
    const SM_CXVSCROLL: i32 = 2;
    let (w96, h96) = window_size_for_client(800, 600, WS_OVERLAPPEDWINDOW, 0, false, 96);
    let (w192, h192) = window_size_for_client(800, 600, WS_OVERLAPPEDWINDOW, 0, false, 192);
    // 枠とタイトルバーの太さはDPIで変わる。クライアント領域より必ず大きく、200%では枠も太い。
    assert!(w96 > 800 && h96 > 600);
    assert!(
        w192 - 800 > w96 - 800 && h192 - 600 > h96 - 600,
        "{w96}x{h96} vs {w192}x{h192}"
    );
    // スクロールバーの幅は200%でおよそ2倍。
    let (a, b) = (
        system_metric(SM_CXVSCROLL, 96),
        system_metric(SM_CXVSCROLL, 192),
    );
    assert!(a > 0 && (b - 2 * a).abs() <= 1, "{a} vs {b}");
}
