//! モニター・ウィンドウのDPIと、`WM_DPICHANGED`の解釈。

use crate::scale;

/// 基準のDPI(100%表示)。
pub const BASE_DPI: u32 = scale::BASE_DPI;

/// `WM_DPICHANGED`のメッセージ番号。
pub const WM_DPICHANGED: u32 = 0x02E0;

/// 矩形。Windowsの`RECT`と**同じメモリ配置**(`#[repr(C)]`)なので、`WM_DPICHANGED`の`lparam`をそのまま読める。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// 1枚のモニターの情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    /// `HMONITOR`の値。
    pub handle: isize,
    /// デバイス名(例: `\\.\DISPLAY1`)。
    pub device_name: String,
    /// 仮想画面上の位置と大きさ(物理ピクセル)。
    pub rect: Rect,
    /// タスクバーなどを除いた作業領域。
    pub work_rect: Rect,
    pub primary: bool,
    pub dpi_x: u32,
    pub dpi_y: u32,
}

impl MonitorInfo {
    /// 拡大率(1.0=100%)。
    pub fn scale(&self) -> f64 {
        scale::factor(self.dpi_x)
    }
    /// 拡大率のパーセント(100/125/150/175/200...)。
    pub fn percent(&self) -> u32 {
        scale::percent(self.dpi_x)
    }
}

/// すべてのモニター。**`init()`の後に呼ぶこと**(DPI非対応のプロセスでは、どのモニターも96dpiと報告される)。
/// Windows以外では空。
pub fn monitors() -> Vec<MonitorInfo> {
    imp::monitors()
}

/// システム(メインモニター)のDPI。取得できなければ96。
pub fn system_dpi() -> u32 {
    imp::system_dpi()
}

/// ウィンドウのDPI(`HWND`の値)。DPI認識が`PerMonitor`以上でないと意味のある値にならない。取得できなければ`None`。
pub fn window_dpi(hwnd: isize) -> Option<u32> {
    imp::window_dpi(hwnd)
}

/// `WM_DPICHANGED`の内容。ウィンドウが別のDPIのモニターへ移った(または設定が変わった)ときに届く。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DpiChanged {
    pub dpi_x: u32,
    pub dpi_y: u32,
    /// Windowsが提案する、新しいDPIでのウィンドウの位置と大きさ。`SetWindowPos`にそのまま渡すのが基本。
    pub suggested: Rect,
}

impl DpiChanged {
    /// `wparam`の下位16bit=X方向のDPI、上位16bit=Y方向のDPI。
    pub fn from_parts(wparam: usize, suggested: Rect) -> Self {
        DpiChanged {
            dpi_x: (wparam & 0xFFFF) as u32,
            dpi_y: ((wparam >> 16) & 0xFFFF) as u32,
            suggested,
        }
    }

    /// ウィンドウプロシージャの`wparam`/`lparam`から読み取る。
    ///
    /// # Safety
    /// `lparam`は`WM_DPICHANGED`で渡された、有効な`RECT`へのポインタでなければならない。
    pub unsafe fn from_message(wparam: usize, lparam: isize) -> Option<Self> {
        let p = lparam as *const Rect;
        if p.is_null() {
            return None;
        }
        // SAFETY: 呼び出し側が有効な`RECT`であることを保証している(`Rect`は`RECT`と同じ並び)。
        Some(Self::from_parts(wparam, *p))
    }

    /// 拡大率のパーセント。
    pub fn percent(&self) -> u32 {
        scale::percent(self.dpi_x)
    }
}

/// クライアント領域の大きさ(物理ピクセル)から、`dpi`でのウィンドウ全体の大きさ`(幅, 高さ)`を求める。
/// 枠・タイトルバー・メニューのDPIによる太さを考慮する(古いWindowsでは従来の計算に代替)。
pub fn window_size_for_client(
    client_w: i32,
    client_h: i32,
    style: u32,
    ex_style: u32,
    has_menu: bool,
    dpi: u32,
) -> (i32, i32) {
    imp::window_size_for_client(client_w, client_h, style, ex_style, has_menu, dpi)
}

/// `dpi`でのシステムの寸法(`SM_*`)。例: スクロールバーの幅`SM_CXVSCROLL = 2`。
pub fn system_metric(index: i32, dpi: u32) -> i32 {
    imp::system_metric(index, dpi)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::sys;

    fn rect(r: sys::RawRect) -> Rect {
        Rect {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        let sys_dpi = sys::system_dpi().unwrap_or(BASE_DPI);
        sys::monitor_handles()
            .into_iter()
            .filter_map(|h| {
                let m = sys::monitor_info(h)?;
                let (dpi_x, dpi_y) = sys::monitor_dpi(h).unwrap_or((sys_dpi, sys_dpi));
                Some(MonitorInfo {
                    handle: h,
                    device_name: m.device,
                    rect: rect(m.rect),
                    work_rect: rect(m.work),
                    primary: m.primary,
                    dpi_x,
                    dpi_y,
                })
            })
            .collect()
    }

    pub fn system_dpi() -> u32 {
        sys::system_dpi().unwrap_or(BASE_DPI)
    }

    pub fn window_dpi(hwnd: isize) -> Option<u32> {
        sys::window_dpi(hwnd)
    }

    pub fn window_size_for_client(
        client_w: i32,
        client_h: i32,
        style: u32,
        ex_style: u32,
        has_menu: bool,
        dpi: u32,
    ) -> (i32, i32) {
        let mut r = sys::RawRect {
            left: 0,
            top: 0,
            right: client_w,
            bottom: client_h,
        };
        sys::adjust_window_rect(&mut r, style, has_menu, ex_style, dpi);
        (r.right - r.left, r.bottom - r.top)
    }

    pub fn system_metric(index: i32, dpi: u32) -> i32 {
        sys::metrics_for_dpi(index, dpi)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn monitors() -> Vec<MonitorInfo> {
        Vec::new()
    }
    pub fn system_dpi() -> u32 {
        BASE_DPI
    }
    pub fn window_dpi(_hwnd: isize) -> Option<u32> {
        None
    }
    pub fn window_size_for_client(
        client_w: i32,
        client_h: i32,
        _style: u32,
        _ex_style: u32,
        _has_menu: bool,
        _dpi: u32,
    ) -> (i32, i32) {
        (client_w, client_h)
    }
    pub fn system_metric(_index: i32, _dpi: u32) -> i32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_changed_splits_wparam_into_x_and_y() {
        let r = Rect {
            left: 10,
            top: 20,
            right: 810,
            bottom: 620,
        };
        let d = DpiChanged::from_parts((144usize << 16) | 120, r);
        assert_eq!((d.dpi_x, d.dpi_y), (120, 144));
        assert_eq!(d.suggested.width(), 800);
        assert_eq!(d.suggested.height(), 600);
        assert_eq!(d.percent(), 125);
    }

    #[test]
    fn dpi_changed_reads_the_suggested_rect_from_lparam() {
        let r = Rect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        let d = unsafe { DpiChanged::from_message(192 | (192 << 16), &r as *const Rect as isize) }
            .unwrap();
        assert_eq!(d.suggested, r);
        assert!(unsafe { DpiChanged::from_message(96, 0) }.is_none());
    }

    #[test]
    fn monitor_info_scale_helpers() {
        let m = MonitorInfo {
            handle: 1,
            device_name: "x".into(),
            rect: Rect::default(),
            work_rect: Rect::default(),
            primary: true,
            dpi_x: 168,
            dpi_y: 168,
        };
        assert_eq!(m.percent(), 175);
        assert!((m.scale() - 1.75).abs() < 1e-9);
    }

    #[test]
    fn rect_has_the_same_layout_as_the_windows_rect() {
        assert_eq!(std::mem::size_of::<Rect>(), 16);
        assert_eq!(std::mem::align_of::<Rect>(), 4);
        let r = Rect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        // SAFETY: `#[repr(C)]`の4つの`i32`なので、同じ並びの配列として読める。
        let a: [i32; 4] = unsafe { std::mem::transmute(r) };
        assert_eq!(a, [1, 2, 3, 4]);
    }

    #[test]
    fn wm_dpichanged_number_matches_the_windows_header() {
        assert_eq!(WM_DPICHANGED, 0x02E0);
    }
}
