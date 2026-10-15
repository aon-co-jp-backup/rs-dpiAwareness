//! DPI認識レベルの種類と、設定・取得(段階的な代替つき)。

use std::fmt;
use std::str::FromStr;

/// DPI認識レベル(強いものほど後ろ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Awareness {
    /// 非対応。Windowsが画面ごと拡大する(ぼやける)。
    Unaware,
    /// 非対応のままだが、GDIの描画だけ高DPI向けに拡大する(Windows 10 1809以降)。
    UnawareGdiScaled,
    /// 起動時のメインモニターのDPIに合わせる。他のモニターへ動かすとぼやける。
    System,
    /// モニターごとのDPIに対応(Windows 8.1以降)。ただし非クライアント領域などは自動で拡大されない。
    PerMonitor,
    /// モニターごとのDPIに完全対応(Windows 10 1703以降)。非クライアント領域・ダイアログ・コモンコントロールも自動で拡大される。推奨。
    PerMonitorV2,
}

impl Awareness {
    /// 強さの順位(`Unaware`と`UnawareGdiScaled`は同順位)。
    pub fn rank(self) -> u8 {
        match self {
            Awareness::Unaware | Awareness::UnawareGdiScaled => 0,
            Awareness::System => 1,
            Awareness::PerMonitor => 2,
            Awareness::PerMonitorV2 => 3,
        }
    }

    /// モニターごとのDPIに対応しているか(`PerMonitor`以上)。
    pub fn is_per_monitor(self) -> bool {
        self.rank() >= 2
    }

    /// マニフェストの`<dpiAwareness>`に書く値。
    pub fn manifest_name(self) -> &'static str {
        match self {
            Awareness::Unaware | Awareness::UnawareGdiScaled => "unaware",
            Awareness::System => "system",
            Awareness::PerMonitor => "PerMonitor",
            Awareness::PerMonitorV2 => "PerMonitorV2",
        }
    }
}

impl fmt::Display for Awareness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Awareness::Unaware => "Unaware",
            Awareness::UnawareGdiScaled => "UnawareGdiScaled",
            Awareness::System => "System",
            Awareness::PerMonitor => "PerMonitor",
            Awareness::PerMonitorV2 => "PerMonitorV2",
        })
    }
}

impl FromStr for Awareness {
    type Err = String;
    /// `v2`/`permonitorv2`/`pmv2`、`pm`/`permonitor`、`system`、`unaware`、`gdi`などを受け付ける(大文字小文字・`-`・`_`は無視)。
    fn from_str(s: &str) -> Result<Self, String> {
        let k: String = s
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ' '))
            .flat_map(|c| c.to_lowercase())
            .collect();
        match k.as_str() {
            "v2" | "pmv2" | "permonitorv2" => Ok(Awareness::PerMonitorV2),
            "pm" | "v1" | "permonitor" => Ok(Awareness::PerMonitor),
            "system" | "sys" => Ok(Awareness::System),
            "unaware" | "none" | "off" => Ok(Awareness::Unaware),
            "gdi" | "gdiscaled" | "unawaregdiscaled" => Ok(Awareness::UnawareGdiScaled),
            _ => Err(format!("認識できないDPI認識レベルです: {s}(v2 / pm / system / unaware / gdi) / unknown awareness: {s}")),
        }
    }
}

/// どのAPIで設定できたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `SetProcessDpiAwarenessContext`(Windows 10 1607以降。`PerMonitorV2`はここからだけ)。
    ContextApi,
    /// `SetProcessDpiAwareness`(Windows 8.1以降)。
    ShcoreApi,
    /// `SetProcessDPIAware`(Windows Vista以降。システム認識のみ)。
    LegacyApi,
    /// 何も呼んでいない(既に決まっていた、または設定できなかった)。
    None,
}

/// 結果の由来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// この呼び出しで設定した。
    Applied,
    /// 既に決まっていた(マニフェストの宣言、または先の呼び出し)。**Windowsはプロセス起動後の変更を許さない**。
    AlreadySet,
    /// 設定できなかった(Windows以外、または全てのAPIが失敗)。
    Unavailable,
}

/// [`init`]/[`set`]の結果。必ず「実際にどうなったか」を再取得して入れる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// 頼んだレベル。
    pub requested: Awareness,
    /// 実際のレベル(設定後に再取得した値)。
    pub achieved: Awareness,
    pub method: Method,
    pub source: Source,
}

impl Outcome {
    /// 頼んだレベル以上になったか。
    pub fn is_satisfied(&self) -> bool {
        self.achieved.rank() >= self.requested.rank()
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ja, en) = match (self.source, self.method) {
            (Source::Applied, m) => ("設定しました".to_string(), format!("applied via {m:?}")),
            (Source::AlreadySet, _) => (
                "既に決まっていました(マニフェストまたは先の呼び出し)".to_string(),
                "already set by the manifest or an earlier call".to_string(),
            ),
            (Source::Unavailable, _) => ("設定できません".to_string(), "unavailable".to_string()),
        };
        write!(
            f,
            "DPI認識: {}(希望 {}、{ja}) / DPI awareness: {} (requested {}, {en})",
            self.achieved, self.requested, self.achieved, self.requested
        )
    }
}

/// プロセスのDPI認識を、可能な限り強い`PerMonitorV2`にする。古いWindowsでは`PerMonitor`→`System`へ段階的に代替する。
///
/// - **ウィンドウを作る前**に呼ぶこと(作った後に変えても既存のウィンドウには効かない)。
/// - 何度呼んでも安全(2回目以降は`Source::AlreadySet`)。
/// - マニフェストで宣言済みなら、その宣言が優先される(宣言のほうが確実なので推奨。[`crate::build`]参照)。
pub fn init() -> Outcome {
    set(Awareness::PerMonitorV2)
}

/// プロセスのDPI認識を`requested`にする(強いレベルが無ければ段階的に代替)。
pub fn set(requested: Awareness) -> Outcome {
    imp::set(requested)
}

/// 今のプロセス(このスレッド)のDPI認識を取得する。Windows以外では常に`Unaware`。
///
/// 注意: 取得するのは**スレッドの**認識コンテキストで、通常はプロセスの設定と同じ。
/// `SetThreadDpiAwarenessContext`で上書きしているスレッドでは、そちらの値になる。
pub fn process_awareness() -> Awareness {
    imp::process_awareness()
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::sys::{self, SysError};

    pub fn process_awareness() -> Awareness {
        if let Some(ctx) = sys::thread_context() {
            // 返るのは擬似ハンドルとは限らないので、`AreDpiAwarenessContextsEqual`で比べる。
            for (c, a) in [
                (sys::CTX_PER_MONITOR_V2, Awareness::PerMonitorV2),
                (sys::CTX_PER_MONITOR, Awareness::PerMonitor),
                (sys::CTX_SYSTEM, Awareness::System),
                (sys::CTX_UNAWARE_GDISCALED, Awareness::UnawareGdiScaled),
                (sys::CTX_UNAWARE, Awareness::Unaware),
            ] {
                if sys::contexts_equal(ctx, c) == Some(true) {
                    return a;
                }
            }
        }
        if let Some(level) = sys::get_shcore() {
            return match level {
                2 => Awareness::PerMonitor,
                1 => Awareness::System,
                _ => Awareness::Unaware,
            };
        }
        if sys::legacy_is_aware() == Some(true) {
            Awareness::System
        } else {
            Awareness::Unaware
        }
    }

    /// 試す順番。`(使うAPI, 値)`。
    fn ladder(requested: Awareness) -> Vec<(Method, i32)> {
        match requested {
            Awareness::PerMonitorV2 => vec![
                (Method::ContextApi, sys::CTX_PER_MONITOR_V2 as i32),
                (Method::ContextApi, sys::CTX_PER_MONITOR as i32),
                (Method::ShcoreApi, 2),
                (Method::LegacyApi, 0),
            ],
            Awareness::PerMonitor => vec![
                (Method::ContextApi, sys::CTX_PER_MONITOR as i32),
                (Method::ShcoreApi, 2),
                (Method::LegacyApi, 0),
            ],
            Awareness::System => vec![
                (Method::ContextApi, sys::CTX_SYSTEM as i32),
                (Method::ShcoreApi, 1),
                (Method::LegacyApi, 0),
            ],
            Awareness::UnawareGdiScaled => {
                vec![(Method::ContextApi, sys::CTX_UNAWARE_GDISCALED as i32)]
            }
            Awareness::Unaware => vec![
                (Method::ContextApi, sys::CTX_UNAWARE as i32),
                (Method::ShcoreApi, 0),
            ],
        }
    }

    pub fn set(requested: Awareness) -> Outcome {
        let before = process_awareness();
        // 既に十分なレベルなら何もしない(Windowsは一度決まった認識を変えさせない)。
        if requested.rank() > 0 && before.rank() >= requested.rank() {
            return Outcome {
                requested,
                achieved: before,
                method: Method::None,
                source: Source::AlreadySet,
            };
        }
        for (method, value) in ladder(requested) {
            let r = match method {
                Method::ContextApi => sys::set_context(value as isize),
                Method::ShcoreApi => sys::set_shcore(value),
                Method::LegacyApi => sys::legacy_set(),
                Method::None => continue,
            };
            match r {
                Ok(()) => {
                    // 設定したつもりにならず、実際の値を再取得して報告する。
                    return Outcome {
                        requested,
                        achieved: process_awareness(),
                        method,
                        source: Source::Applied,
                    };
                }
                Err(SysError::Code(c))
                    if c == sys::ERROR_ACCESS_DENIED || c == sys::E_ACCESSDENIED =>
                {
                    // 既に決まっている(マニフェスト、または先の呼び出し)。これ以上試しても変えられない。
                    return Outcome {
                        requested,
                        achieved: process_awareness(),
                        method: Method::None,
                        source: Source::AlreadySet,
                    };
                }
                // そのAPIや値がこのWindowsに無い。次の代替へ。
                Err(_) => continue,
            }
        }
        Outcome {
            requested,
            achieved: process_awareness(),
            method: Method::None,
            source: Source::Unavailable,
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn process_awareness() -> Awareness {
        Awareness::Unaware
    }

    pub fn set(requested: Awareness) -> Outcome {
        Outcome {
            requested,
            achieved: Awareness::Unaware,
            method: Method::None,
            source: Source::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_orders_the_levels() {
        assert!(Awareness::PerMonitorV2.rank() > Awareness::PerMonitor.rank());
        assert!(Awareness::PerMonitor.rank() > Awareness::System.rank());
        assert!(Awareness::System.rank() > Awareness::Unaware.rank());
        assert_eq!(
            Awareness::Unaware.rank(),
            Awareness::UnawareGdiScaled.rank()
        );
        assert!(Awareness::PerMonitor.is_per_monitor() && !Awareness::System.is_per_monitor());
    }

    #[test]
    fn parses_friendly_names() {
        for (s, a) in [
            ("v2", Awareness::PerMonitorV2),
            ("Per-Monitor-V2", Awareness::PerMonitorV2),
            ("PMV2", Awareness::PerMonitorV2),
            ("pm", Awareness::PerMonitor),
            ("system", Awareness::System),
            ("UNAWARE", Awareness::Unaware),
            ("gdi", Awareness::UnawareGdiScaled),
        ] {
            assert_eq!(s.parse::<Awareness>().unwrap(), a, "{s}");
        }
        assert!("nonsense".parse::<Awareness>().is_err());
    }

    #[test]
    fn display_round_trips_through_parse() {
        for a in [
            Awareness::Unaware,
            Awareness::UnawareGdiScaled,
            Awareness::System,
            Awareness::PerMonitor,
            Awareness::PerMonitorV2,
        ] {
            assert_eq!(a.to_string().parse::<Awareness>().unwrap(), a);
        }
    }

    #[test]
    fn outcome_display_is_readable_in_both_languages() {
        let o = Outcome {
            requested: Awareness::PerMonitorV2,
            achieved: Awareness::PerMonitor,
            method: Method::ShcoreApi,
            source: Source::Applied,
        };
        let t = o.to_string();
        assert!(
            t.contains("DPI認識: PerMonitor(希望 PerMonitorV2、設定しました)"),
            "{t}"
        );
        assert!(
            t.contains("DPI awareness: PerMonitor (requested PerMonitorV2, applied via ShcoreApi)"),
            "{t}"
        );
    }

    #[test]
    fn outcome_satisfaction_compares_ranks() {
        let o = |req, ach| Outcome {
            requested: req,
            achieved: ach,
            method: Method::None,
            source: Source::AlreadySet,
        };
        assert!(o(Awareness::PerMonitorV2, Awareness::PerMonitorV2).is_satisfied());
        assert!(o(Awareness::System, Awareness::PerMonitorV2).is_satisfied());
        assert!(!o(Awareness::PerMonitorV2, Awareness::System).is_satisfied());
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_is_a_harmless_no_op() {
        let o = init();
        assert_eq!(o.source, Source::Unavailable);
        assert_eq!(process_awareness(), Awareness::Unaware);
        assert!(!crate::is_supported());
    }
}
