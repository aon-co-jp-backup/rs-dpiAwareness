//! アプリケーションマニフェスト(`<dpiAware>`/`<dpiAwareness>`)の生成・解析・EXEからの読み出し。
//!
//! DPI認識は**マニフェストで宣言するのが最も確実**(プロセスが始まる前に決まるので、最初のウィンドウや
//! 他のライブラリが先にDPIに触れても取りこぼさない)。実行時の[`crate::init`]は、マニフェストが無い場合の保険として使う。

use crate::awareness::Awareness;
use std::fmt;

/// 生成するマニフェストの設定。`Manifest::new(Awareness::PerMonitorV2)`から始めて、必要なものだけ足す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub awareness: Awareness,
    /// Windows 10/11対応を宣言する(既定: `true`)。宣言しないと、古いOSとして扱われる機能がある。
    pub windows10_compat: bool,
    /// コモンコントロールv6(見た目の新しいボタン等)を使う。使うには`InitCommonControlsEx`が要る。
    pub common_controls_v6: bool,
    /// 260文字を超えるパスを扱えるようにする(`longPathAware`)。
    pub long_path_aware: bool,
    /// ANSI系APIの文字コードをUTF-8にする(`activeCodePage`、Windows 10 1903以降)。
    pub utf8_code_page: bool,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest::new(Awareness::PerMonitorV2)
    }
}

impl Manifest {
    pub fn new(awareness: Awareness) -> Self {
        Manifest {
            awareness,
            windows10_compat: true,
            common_controls_v6: false,
            long_path_aware: false,
            utf8_code_page: false,
        }
    }
    pub fn common_controls_v6(mut self, on: bool) -> Self {
        self.common_controls_v6 = on;
        self
    }
    pub fn long_path_aware(mut self, on: bool) -> Self {
        self.long_path_aware = on;
        self
    }
    pub fn utf8_code_page(mut self, on: bool) -> Self {
        self.utf8_code_page = on;
        self
    }
    pub fn windows10_compat(mut self, on: bool) -> Self {
        self.windows10_compat = on;
        self
    }

    /// マニフェストのXML。古いWindowsでも効くよう、`<dpiAware>`(8.1以降向け)と`<dpiAwareness>`(10 1607以降向け)の両方を書く。
    pub fn to_xml(&self) -> String {
        let (aware, awareness) = match self.awareness {
            Awareness::PerMonitorV2 => ("true/pm", "PerMonitorV2,PerMonitor"),
            Awareness::PerMonitor => ("true/pm", "PerMonitor"),
            Awareness::System => ("true", "system"),
            Awareness::Unaware | Awareness::UnawareGdiScaled => ("false", "unaware"),
        };
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
        s.push_str(
            "<assembly xmlns=\"urn:schemas-microsoft-com:asm.v1\" manifestVersion=\"1.0\">\n",
        );
        if self.windows10_compat {
            s.push_str("  <compatibility xmlns=\"urn:schemas-microsoft-com:compatibility.v1\">\n    <application>\n");
            s.push_str("      <supportedOS Id=\"{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}\"/>\n");
            s.push_str("    </application>\n  </compatibility>\n");
        }
        s.push_str(
            "  <application xmlns=\"urn:schemas-microsoft-com:asm.v3\">\n    <windowsSettings>\n",
        );
        s.push_str(&format!("      <dpiAware xmlns=\"http://schemas.microsoft.com/SMI/2005/WindowsSettings\">{aware}</dpiAware>\n"));
        s.push_str(&format!("      <dpiAwareness xmlns=\"http://schemas.microsoft.com/SMI/2016/WindowsSettings\">{awareness}</dpiAwareness>\n"));
        if self.awareness == Awareness::UnawareGdiScaled {
            s.push_str("      <gdiScaling xmlns=\"http://schemas.microsoft.com/SMI/2017/WindowsSettings\">true</gdiScaling>\n");
        }
        if self.long_path_aware {
            s.push_str("      <longPathAware xmlns=\"http://schemas.microsoft.com/SMI/2016/WindowsSettings\">true</longPathAware>\n");
        }
        if self.utf8_code_page {
            s.push_str("      <activeCodePage xmlns=\"http://schemas.microsoft.com/SMI/2019/WindowsSettings\">UTF-8</activeCodePage>\n");
        }
        s.push_str("    </windowsSettings>\n  </application>\n");
        if self.common_controls_v6 {
            s.push_str("  <dependency>\n    <dependentAssembly>\n");
            s.push_str("      <assemblyIdentity type=\"win32\" name=\"Microsoft.Windows.Common-Controls\" version=\"6.0.0.0\" processorArchitecture=\"*\" publicKeyToken=\"6595b64144ccf1df\" language=\"*\"/>\n");
            s.push_str("    </dependentAssembly>\n  </dependency>\n");
        }
        s.push_str("</assembly>\n");
        s
    }
}

/// 既存のマニフェストが**宣言している**内容(解析結果)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declared {
    /// `<dpiAware>`の値(`true`/`false`/`true/pm`/`per monitor`など)。
    pub dpi_aware: Option<String>,
    /// `<dpiAwareness>`のカンマ区切りの値(先に書いてあるほど優先)。
    pub dpi_awareness: Vec<String>,
    pub gdi_scaling: bool,
    pub long_path_aware: bool,
    pub utf8_code_page: bool,
    pub common_controls_v6: bool,
}

/// 開始タグの名前(接頭辞を除く)とテキストを順に取り出す。コメント・宣言・終了タグは飛ばす。
fn elements(xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        if rest.starts_with("!--") {
            // コメントの中のタグは読まない。
            rest = rest.find("-->").map_or("", |e| &rest[e + 3..]);
            continue;
        }
        if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
            continue;
        }
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..end];
        let name_end = tag
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(tag.len());
        let name = &tag[..name_end];
        let local = name.rsplit(':').next().unwrap_or(name).to_string();
        let after = &rest[end + 1..];
        let text = if tag.ends_with('/') {
            String::new()
        } else {
            after.split('<').next().unwrap_or("").trim().to_string()
        };
        out.push((local, text));
    }
    out
}

impl Declared {
    /// マニフェストのXMLを解析する(必要な要素だけを読む簡易な解析。XML全体の正当性は検査しない)。
    pub fn parse(xml: &str) -> Declared {
        let mut d = Declared::default();
        let mut raw_attrs_cc = false;
        for (name, text) in elements(xml) {
            match name.as_str() {
                "dpiAware" => d.dpi_aware = Some(text),
                "dpiAwareness" => {
                    d.dpi_awareness = text
                        .split(',')
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect()
                }
                "gdiScaling" => d.gdi_scaling = text.eq_ignore_ascii_case("true"),
                "longPathAware" => d.long_path_aware = text.eq_ignore_ascii_case("true"),
                "activeCodePage" => d.utf8_code_page = text.eq_ignore_ascii_case("utf-8"),
                "assemblyIdentity" => raw_attrs_cc = true,
                _ => {}
            }
        }
        d.common_controls_v6 = raw_attrs_cc
            && xml.contains("Microsoft.Windows.Common-Controls")
            && xml.contains("version=\"6.");
        d
    }

    /// Windows 10 1607以降が実際に採用するレベル。`<dpiAwareness>`の**最初に認識できた値**が優先され、
    /// 無ければ`<dpiAware>`を見る。どちらも無ければ`None`(=非対応として扱われる)。
    pub fn effective(&self) -> Option<Awareness> {
        for v in &self.dpi_awareness {
            match v.to_ascii_lowercase().as_str() {
                "permonitorv2" => return Some(Awareness::PerMonitorV2),
                "permonitor" => return Some(Awareness::PerMonitor),
                "system" => return Some(Awareness::System),
                "unaware" => {
                    return Some(if self.gdi_scaling {
                        Awareness::UnawareGdiScaled
                    } else {
                        Awareness::Unaware
                    })
                }
                _ => continue,
            }
        }
        self.legacy()
    }

    /// `<dpiAwareness>`を知らない古いWindows(8.1〜10 1511)が採用するレベル。
    pub fn legacy(&self) -> Option<Awareness> {
        match self
            .dpi_aware
            .as_deref()?
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "true/pm" | "per monitor" | "permonitor" => Some(Awareness::PerMonitor),
            "true" => Some(Awareness::System),
            "false" => Some(Awareness::Unaware),
            _ => None,
        }
    }

    /// 宣言が無い場合も含めた最終的なレベル(宣言が無ければ`Unaware`)。
    pub fn effective_or_default(&self) -> Awareness {
        self.effective().unwrap_or(Awareness::Unaware)
    }
}

/// マニフェストの読み出しの失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// このOSでは読み出せない(Windows以外)。
    Unsupported,
    /// ファイルを開けない(Win32のエラーコード。2=見つからない、193=実行ファイルではない、など)。
    Os(u32),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Unsupported => f.write_str("EXEのマニフェストの読み出しはWindowsのみ対応です / reading an EXE manifest is Windows-only"),
            ManifestError::Os(2) => f.write_str("ファイルが見つかりません / file not found"),
            ManifestError::Os(193) => f.write_str("Windowsの実行ファイル(EXE/DLL)ではありません / not a Windows executable"),
            ManifestError::Os(c) => write!(f, "ファイルを開けません(Win32エラー {c}) / cannot open the file (Win32 error {c})"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// EXE/DLLに**埋め込まれた**マニフェストのXML。埋め込まれていなければ`Ok(None)`。コードは実行しない(データとして読むだけ)。
pub fn read_embedded_manifest(path: &str) -> Result<Option<String>, ManifestError> {
    imp::read(path)
}

#[cfg(windows)]
mod imp {
    use super::*;
    pub fn read(path: &str) -> Result<Option<String>, ManifestError> {
        match crate::sys::embedded_manifest(path) {
            Ok(Some(bytes)) => {
                let b = bytes
                    .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
                    .unwrap_or(&bytes);
                Ok(Some(String::from_utf8_lossy(b).into_owned()))
            }
            Ok(None) => Ok(None),
            Err(crate::sys::SysError::Code(c)) => Err(ManifestError::Os(c)),
            Err(crate::sys::SysError::Unavailable) => Err(ManifestError::Unsupported),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn read(_path: &str) -> Result<Option<String>, ManifestError> {
        Err(ManifestError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Awareness; 5] = [
        Awareness::Unaware,
        Awareness::UnawareGdiScaled,
        Awareness::System,
        Awareness::PerMonitor,
        Awareness::PerMonitorV2,
    ];

    #[test]
    fn generated_manifest_declares_both_elements_for_per_monitor_v2() {
        let x = Manifest::default().to_xml();
        assert!(x.contains(">true/pm</dpiAware>"), "{x}");
        assert!(x.contains(">PerMonitorV2,PerMonitor</dpiAwareness>"), "{x}");
        assert!(x.contains("{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"));
        assert!(
            !x.contains("Common-Controls")
                && !x.contains("longPathAware")
                && !x.contains("activeCodePage")
        );
    }

    #[test]
    fn what_we_generate_is_what_we_read_back() {
        for a in ALL {
            let d = Declared::parse(&Manifest::new(a).to_xml());
            assert_eq!(d.effective(), Some(a), "{a}");
        }
    }

    #[test]
    fn optional_features_round_trip() {
        let x = Manifest::new(Awareness::PerMonitorV2)
            .common_controls_v6(true)
            .long_path_aware(true)
            .utf8_code_page(true)
            .to_xml();
        let d = Declared::parse(&x);
        assert!(d.common_controls_v6 && d.long_path_aware && d.utf8_code_page);
        let plain = Declared::parse(&Manifest::default().to_xml());
        assert!(!plain.common_controls_v6 && !plain.long_path_aware && !plain.utf8_code_page);
    }

    #[test]
    fn dpi_aware_is_not_confused_with_dpi_awareness() {
        let only_old = r#"<assembly><application><windowsSettings><dpiAware xmlns="x">true</dpiAware></windowsSettings></application></assembly>"#;
        let d = Declared::parse(only_old);
        assert_eq!(d.dpi_aware.as_deref(), Some("true"));
        assert!(d.dpi_awareness.is_empty());
        assert_eq!(d.effective(), Some(Awareness::System));
    }

    #[test]
    fn first_recognised_dpi_awareness_value_wins() {
        let x = r#"<windowsSettings><dpiAware>true</dpiAware><dpiAwareness>bogus, PerMonitor ,PerMonitorV2</dpiAwareness></windowsSettings>"#;
        assert_eq!(Declared::parse(x).effective(), Some(Awareness::PerMonitor));
    }

    #[test]
    fn legacy_view_ignores_dpi_awareness() {
        let d = Declared::parse(&Manifest::default().to_xml());
        assert_eq!(d.legacy(), Some(Awareness::PerMonitor));
        assert_eq!(d.effective(), Some(Awareness::PerMonitorV2));
    }

    #[test]
    fn no_declaration_means_unaware() {
        let d = Declared::parse("<assembly/>");
        assert_eq!(d.effective(), None);
        assert_eq!(d.effective_or_default(), Awareness::Unaware);
    }

    #[test]
    fn gdi_scaling_distinguishes_the_unaware_variants() {
        assert_eq!(
            Declared::parse(&Manifest::new(Awareness::UnawareGdiScaled).to_xml()).effective(),
            Some(Awareness::UnawareGdiScaled)
        );
        assert_eq!(
            Declared::parse(&Manifest::new(Awareness::Unaware).to_xml()).effective(),
            Some(Awareness::Unaware)
        );
    }

    #[test]
    fn namespace_prefixes_and_comments_are_tolerated() {
        let x = r#"<?xml version="1.0"?><!-- <dpiAware>false</dpiAware> --><a:assembly><ws:dpiAwareness>system</ws:dpiAwareness></a:assembly>"#;
        let d = Declared::parse(x);
        assert_eq!(d.effective(), Some(Awareness::System));
    }

    #[cfg(not(windows))]
    #[test]
    fn reading_an_exe_manifest_is_unsupported_off_windows() {
        assert_eq!(read_embedded_manifest("x"), Err(ManifestError::Unsupported));
    }
}
