//! Windows APIの呼び出し層(内部用)。
//!
//! **新しいAPIは実行時に`GetProcAddress`で探す**(静的リンクにしない)。静的にリンクすると、そのAPIが無い古いWindows
//! (例: `SetProcessDpiAwarenessContext`はWindows 10 1607以降、`GetDpiForMonitor`はWindows 8.1以降)でEXEが起動すらしなくなる。
//! Microsoftのドキュメントも、新しいDPI関数を使うときは動的に解決するよう勧めている。

use std::ffi::c_void;

pub type Hwnd = isize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysError {
    /// そのAPIがこのWindowsに無い。
    Unavailable,
    /// APIが失敗した(Win32のエラーコード、またはHRESULT)。
    Code(u32),
}

pub const ERROR_ACCESS_DENIED: u32 = 5;
pub const E_ACCESSDENIED: u32 = 0x8007_0005;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RawRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MonitorInfoExW {
    pub cb_size: u32,
    pub rc_monitor: RawRect,
    pub rc_work: RawRect,
    pub flags: u32,
    pub device: [u16; 32],
}

pub const MONITORINFOF_PRIMARY: u32 = 1;

type MonitorEnumProc = unsafe extern "system" fn(isize, isize, *mut RawRect, isize) -> i32;

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
    fn LoadLibraryW(name: *const u16) -> isize;
    fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
    fn FreeLibrary(module: isize) -> i32;
    fn GetProcAddress(module: isize, name: *const u8) -> *const c_void;
    fn GetLastError() -> u32;
    fn FindResourceW(module: isize, name: *const u16, kind: *const u16) -> isize;
    fn LoadResource(module: isize, res: isize) -> isize;
    fn LockResource(res: isize) -> *const u8;
    fn SizeofResource(module: isize, res: isize) -> u32;
}

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayMonitors(
        hdc: isize,
        clip: *const RawRect,
        proc_: MonitorEnumProc,
        data: isize,
    ) -> i32;
    fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfoExW) -> i32;
    fn GetDC(hwnd: Hwnd) -> isize;
    fn ReleaseDC(hwnd: Hwnd, hdc: isize) -> i32;
    fn AdjustWindowRectEx(rect: *mut RawRect, style: u32, menu: i32, ex_style: u32) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn GetDeviceCaps(hdc: isize, index: i32) -> i32;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `lib`の関数`name`(NUL終端)を探す。見つからなければ`None`。
fn find<T: Copy>(lib: &str, name: &[u8]) -> Option<T> {
    debug_assert_eq!(name.last(), Some(&0));
    debug_assert_eq!(
        std::mem::size_of::<T>(),
        std::mem::size_of::<*const c_void>()
    );
    let w = wide(lib);
    // SAFETY: 引数はNUL終端の有効な文字列。取得した関数ポインタは、署名が合う型`T`としてだけ使う。
    unsafe {
        let mut h = GetModuleHandleW(w.as_ptr());
        if h == 0 {
            h = LoadLibraryW(w.as_ptr());
        }
        if h == 0 {
            return None;
        }
        let p = GetProcAddress(h, name.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy::<*const c_void, T>(&p))
        }
    }
}

// ── DPI認識の設定・取得 ──────────────────────────────

pub const CTX_UNAWARE: isize = -1;
pub const CTX_SYSTEM: isize = -2;
pub const CTX_PER_MONITOR: isize = -3;
pub const CTX_PER_MONITOR_V2: isize = -4;
pub const CTX_UNAWARE_GDISCALED: isize = -5;

pub fn set_context(ctx: isize) -> Result<(), SysError> {
    let f: unsafe extern "system" fn(isize) -> i32 =
        find("user32.dll", b"SetProcessDpiAwarenessContext\0").ok_or(SysError::Unavailable)?;
    // SAFETY: 擬似ハンドル(定数)を渡すだけ。
    if unsafe { f(ctx) } != 0 {
        Ok(())
    } else {
        Err(SysError::Code(unsafe { GetLastError() }))
    }
}

pub fn thread_context() -> Option<isize> {
    let f: unsafe extern "system" fn() -> isize =
        find("user32.dll", b"GetThreadDpiAwarenessContext\0")?;
    // SAFETY: 引数なし。
    Some(unsafe { f() })
}

pub fn contexts_equal(a: isize, b: isize) -> Option<bool> {
    let f: unsafe extern "system" fn(isize, isize) -> i32 =
        find("user32.dll", b"AreDpiAwarenessContextsEqual\0")?;
    // SAFETY: ハンドルの比較だけ。
    Some(unsafe { f(a, b) } != 0)
}

/// `level`: 0=なし、1=システム、2=モニターごと。
pub fn set_shcore(level: i32) -> Result<(), SysError> {
    let f: unsafe extern "system" fn(i32) -> i32 =
        find("shcore.dll", b"SetProcessDpiAwareness\0").ok_or(SysError::Unavailable)?;
    // SAFETY: 整数を渡すだけ。
    let hr = unsafe { f(level) };
    if hr == 0 {
        Ok(())
    } else {
        Err(SysError::Code(hr as u32))
    }
}

pub fn get_shcore() -> Option<i32> {
    let f: unsafe extern "system" fn(isize, *mut i32) -> i32 =
        find("shcore.dll", b"GetProcessDpiAwareness\0")?;
    let mut level = 0i32;
    // SAFETY: 第1引数0は「現在のプロセス」。出力先は有効な`i32`。
    (unsafe { f(0, &mut level) } == 0).then_some(level)
}

pub fn legacy_set() -> Result<(), SysError> {
    let f: unsafe extern "system" fn() -> i32 =
        find("user32.dll", b"SetProcessDPIAware\0").ok_or(SysError::Unavailable)?;
    // SAFETY: 引数なし。
    if unsafe { f() } != 0 {
        Ok(())
    } else {
        Err(SysError::Code(unsafe { GetLastError() }))
    }
}

pub fn legacy_is_aware() -> Option<bool> {
    let f: unsafe extern "system" fn() -> i32 = find("user32.dll", b"IsProcessDPIAware\0")?;
    // SAFETY: 引数なし。
    Some(unsafe { f() } != 0)
}

// ── DPIの取得 ────────────────────────────────────────

/// 画面のDC経由の昔ながらの方法(全Windowsで使える)。
fn gdi_dpi() -> Option<u32> {
    // SAFETY: GetDC(0)は画面のDC。使い終わったら必ず解放する。
    unsafe {
        let dc = GetDC(0);
        if dc == 0 {
            return None;
        }
        let dpi = GetDeviceCaps(dc, 88); // LOGPIXELSX
        ReleaseDC(0, dc);
        (dpi > 0).then_some(dpi as u32)
    }
}

pub fn system_dpi() -> Option<u32> {
    if let Some(f) = find::<unsafe extern "system" fn() -> u32>("user32.dll", b"GetDpiForSystem\0")
    {
        // SAFETY: 引数なし。
        let d = unsafe { f() };
        if d > 0 {
            return Some(d);
        }
    }
    gdi_dpi()
}

pub fn window_dpi(hwnd: Hwnd) -> Option<u32> {
    let f: unsafe extern "system" fn(Hwnd) -> u32 = find("user32.dll", b"GetDpiForWindow\0")?;
    // SAFETY: 無効なハンドルのときは0が返る。
    let d = unsafe { f(hwnd) };
    (d > 0).then_some(d)
}

pub fn monitor_dpi(monitor: isize) -> Option<(u32, u32)> {
    let f: unsafe extern "system" fn(isize, i32, *mut u32, *mut u32) -> i32 =
        find("shcore.dll", b"GetDpiForMonitor\0")?;
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: 0はMDT_EFFECTIVE_DPI。出力先は有効な`u32`。
    (unsafe { f(monitor, 0, &mut x, &mut y) } == 0 && x > 0 && y > 0).then_some((x, y))
}

// ── モニター ─────────────────────────────────────────

unsafe extern "system" fn collect_monitor(
    monitor: isize,
    _dc: isize,
    _rect: *mut RawRect,
    data: isize,
) -> i32 {
    // SAFETY: `data`は`monitors`が渡した`Vec<isize>`への有効なポインタ。
    let v = &mut *(data as *mut Vec<isize>);
    v.push(monitor);
    1
}

pub fn monitor_handles() -> Vec<isize> {
    let mut v: Vec<isize> = Vec::new();
    // SAFETY: コールバックは同期的に呼ばれ、`v`はこの呼び出しの間生きている。
    unsafe {
        EnumDisplayMonitors(
            0,
            std::ptr::null(),
            collect_monitor,
            &mut v as *mut Vec<isize> as isize,
        );
    }
    v
}

pub struct RawMonitor {
    pub device: String,
    pub rect: RawRect,
    pub work: RawRect,
    pub primary: bool,
}

pub fn monitor_info(monitor: isize) -> Option<RawMonitor> {
    let mut info = MonitorInfoExW {
        cb_size: std::mem::size_of::<MonitorInfoExW>() as u32,
        rc_monitor: RawRect::default(),
        rc_work: RawRect::default(),
        flags: 0,
        device: [0; 32],
    };
    // SAFETY: `cb_size`を設定した有効な構造体を渡す。
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    let n = info
        .device
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(info.device.len());
    Some(RawMonitor {
        device: String::from_utf16_lossy(&info.device[..n]),
        rect: info.rc_monitor,
        work: info.rc_work,
        primary: info.flags & MONITORINFOF_PRIMARY != 0,
    })
}

// ── ウィンドウの大きさ ───────────────────────────────

/// DPIを考慮して、クライアント領域に対するウィンドウ全体の矩形を求める(古いWindowsでは普通の計算に代替)。
pub fn adjust_window_rect(
    rect: &mut RawRect,
    style: u32,
    menu: bool,
    ex_style: u32,
    dpi: u32,
) -> bool {
    if let Some(f) = find::<unsafe extern "system" fn(*mut RawRect, u32, i32, u32, u32) -> i32>(
        "user32.dll",
        b"AdjustWindowRectExForDpi\0",
    ) {
        // SAFETY: `rect`は有効な可変参照。
        return unsafe { f(rect, style, menu as i32, ex_style, dpi) } != 0;
    }
    // SAFETY: 同上。
    unsafe { AdjustWindowRectEx(rect, style, menu as i32, ex_style) != 0 }
}

/// DPIを考慮したシステムのサイズ(スクロールバーの幅など)。古いWindowsでは96dpi換算の値を拡大して代替。
pub fn metrics_for_dpi(index: i32, dpi: u32) -> i32 {
    if let Some(f) = find::<unsafe extern "system" fn(i32, u32) -> i32>(
        "user32.dll",
        b"GetSystemMetricsForDpi\0",
    ) {
        // SAFETY: 整数を渡すだけ。
        return unsafe { f(index, dpi) };
    }
    // SAFETY: 整数を渡すだけ。
    let base = unsafe { GetSystemMetrics(index) };
    let sys = system_dpi().unwrap_or(96);
    crate::scale::rescale(base, sys, dpi)
}

// ── EXEに埋め込まれたマニフェストの読み出し ─────────────

/// `path`のEXE/DLLに埋め込まれたマニフェスト(リソース番号1、無ければ2)のバイト列。
pub fn embedded_manifest(path: &str) -> Result<Option<Vec<u8>>, SysError> {
    const LOAD_LIBRARY_AS_DATAFILE: u32 = 0x2;
    const LOAD_LIBRARY_AS_IMAGE_RESOURCE: u32 = 0x20;
    const RT_MANIFEST: usize = 24;
    let w = wide(path);
    // SAFETY: データファイルとして読み込むだけ(コードは実行しない)。使い終わったら解放する。
    unsafe {
        let module = LoadLibraryExW(
            w.as_ptr(),
            0,
            LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
        );
        if module == 0 {
            return Err(SysError::Code(GetLastError()));
        }
        let mut found = None;
        for id in [1usize, 2] {
            let res = FindResourceW(module, id as *const u16, RT_MANIFEST as *const u16);
            if res == 0 {
                continue;
            }
            let size = SizeofResource(module, res) as usize;
            let handle = LoadResource(module, res);
            if handle == 0 || size == 0 {
                continue;
            }
            let p = LockResource(handle);
            if !p.is_null() {
                found = Some(std::slice::from_raw_parts(p, size).to_vec());
                break;
            }
        }
        FreeLibrary(module);
        Ok(found)
    }
}
