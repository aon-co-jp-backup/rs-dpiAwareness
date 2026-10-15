//! 高DPI対応ウィンドウの最小サンプル(Win32を直接呼ぶ。外部クレート不要)。
//!
//! - 起動直後に `rs_dpi_awareness::init()` を呼ぶ(ウィンドウを作る前)。
//! - 論理サイズ(96dpi基準)の640x480を、そのモニターのDPIに合わせた物理サイズで作る。
//! - `WM_DPICHANGED` でWindowsが提案する矩形に合わせ、タイトルに現在のDPIを出す。
//!
//! 実行: `cargo run --example window`(ウィンドウを別のDPIのモニターへ動かすとタイトルが変わる)
//! 自己診断: `cargo run --example window -- --selftest`(合成した`WM_DPICHANGED`で、ウィンドウが提案どおりに変わるか確かめて終了)
#![cfg(windows)]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use rs_dpi_awareness::{
    init, scale, system_dpi, window_dpi, window_size_for_client, DpiChanged, Rect, WM_DPICHANGED,
};

type HWND = isize;

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    wnd_proc: unsafe extern "system" fn(HWND, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: isize,
    icon: isize,
    cursor: isize,
    background: isize,
    menu: *const u16,
    class_name: *const u16,
    icon_sm: isize,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: [i32; 2],
    private: u32,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(c: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: isize,
        inst: isize,
        param: isize,
    ) -> HWND;
    fn DefWindowProcW(h: HWND, m: u32, w: usize, l: isize) -> isize;
    fn ShowWindow(h: HWND, cmd: i32) -> i32;
    fn SetWindowPos(h: HWND, after: HWND, x: i32, y: i32, w: i32, h2: i32, flags: u32) -> i32;
    fn GetWindowRect(h: HWND, r: *mut Rect) -> i32;
    fn SetWindowTextW(h: HWND, t: *const u16) -> i32;
    fn GetWindowTextW(h: HWND, buf: *mut u16, max: i32) -> i32;
    fn SendMessageW(h: HWND, m: u32, w: usize, l: isize) -> isize;
    fn DestroyWindow(h: HWND) -> i32;
    fn PostQuitMessage(code: i32);
    fn GetMessageW(m: *mut MSG, h: HWND, min: u32, max: u32) -> i32;
    fn TranslateMessage(m: *const MSG) -> i32;
    fn DispatchMessageW(m: *const MSG) -> isize;
    fn LoadCursorW(inst: isize, name: usize) -> isize;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
const SW_SHOWNOACTIVATE: i32 = 4;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const COLOR_WINDOW: isize = 5;
const IDC_ARROW: usize = 32512;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn set_title(hwnd: HWND, dpi: u32) {
    let t = format!("rs-dpiAwareness — DPI {dpi} ({}%)", scale::percent(dpi));
    unsafe { SetWindowTextW(hwnd, wide(&t).as_ptr()) };
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_DPICHANGED => {
            // SAFETY: WM_DPICHANGEDのlparamは、有効なRECTへのポインタ。
            if let Some(d) = unsafe { DpiChanged::from_message(wparam, lparam) } {
                let r = d.suggested;
                unsafe {
                    SetWindowPos(
                        hwnd,
                        0,
                        r.left,
                        r.top,
                        r.width(),
                        r.height(),
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
                set_title(hwnd, d.dpi_x);
            }
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn create_window() -> Result<HWND, String> {
    let class = wide("RsDpiAwarenessExample");
    unsafe {
        let inst = GetModuleHandleW(std::ptr::null());
        let c = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            wnd_proc,
            cls_extra: 0,
            wnd_extra: 0,
            instance: inst,
            icon: 0,
            cursor: LoadCursorW(0, IDC_ARROW),
            background: COLOR_WINDOW + 1,
            menu: std::ptr::null(),
            class_name: class.as_ptr(),
            icon_sm: 0,
        };
        // 同じ名前の登録が既にあっても(テストが複数回呼ぶ場合)そのまま使えるので、戻り値は見ない。
        RegisterClassExW(&c);
        // 論理サイズ(96dpi基準)640x480を、システムDPIの物理サイズへ。
        let dpi = system_dpi();
        let (w, h) = window_size_for_client(
            scale::px(640, dpi),
            scale::px(480, dpi),
            WS_OVERLAPPEDWINDOW,
            0,
            false,
            dpi,
        );
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("rs-dpiAwareness").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            w,
            h,
            0,
            0,
            inst,
            0,
        );
        if hwnd == 0 {
            return Err("ウィンドウを作れません / CreateWindowExW failed".to_string());
        }
        set_title(hwnd, window_dpi(hwnd).unwrap_or(dpi));
        Ok(hwnd)
    }
}

/// 合成した`WM_DPICHANGED`(216dpi=225%、提案矩形つき)をウィンドウへ送り、提案どおりになるか確かめる。
pub fn selftest() -> Result<(), String> {
    init();
    let hwnd = create_window()?;
    unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    let suggested = Rect {
        left: 120,
        top: 80,
        right: 120 + 1480,
        bottom: 80 + 1100,
    };
    let wparam = (216usize << 16) | 216;
    unsafe {
        SendMessageW(
            hwnd,
            WM_DPICHANGED,
            wparam,
            &suggested as *const Rect as isize,
        )
    };
    let mut got = Rect::default();
    unsafe { GetWindowRect(hwnd, &mut got) };
    let mut buf = [0u16; 128];
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    let title = String::from_utf16_lossy(&buf[..n]);
    unsafe { DestroyWindow(hwnd) };
    if got != suggested {
        return Err(format!(
            "ウィンドウが提案の矩形になっていません: 期待 {suggested:?}、実際 {got:?}"
        ));
    }
    if !title.contains("216") || !title.contains("225%") {
        return Err(format!("タイトルが新しいDPIになっていません: {title}"));
    }
    Ok(())
}

fn main() {
    if std::env::args().any(|a| a == "--selftest") {
        match selftest() {
            Ok(()) => println!("selftest ok"),
            Err(e) => {
                eprintln!("selftest failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    println!("{}", init());
    let hwnd = match create_window() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    unsafe { ShowWindow(hwnd, 1) };
    let mut msg = MSG {
        hwnd: 0,
        message: 0,
        wparam: 0,
        lparam: 0,
        time: 0,
        pt: [0; 2],
        private: 0,
    };
    unsafe {
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
