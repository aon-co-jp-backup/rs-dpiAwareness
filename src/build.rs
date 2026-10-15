//! `build.rs`から使う、マニフェストの埋め込み補助。
//!
//! ```ignore
//! // build.rs   (Cargo.tomlに [build-dependencies] rs-dpi-awareness = "0.1")
//! fn main() {
//!     rs_dpi_awareness::build::embed(rs_dpi_awareness::Awareness::PerMonitorV2);
//! }
//! ```
//!
//! これだけで、MSVCターゲットのEXEにDPI認識のマニフェストが埋め込まれる(`link.exe`の`/MANIFEST:EMBED`を使う)。
//! Windows以外のターゲットでは何もしない。**MSVC以外のWindowsターゲット(`*-pc-windows-gnu`)には未対応**で、
//! ビルドは失敗させず警告を出す(その場合は実行時の[`crate::init`]を使うこと)。

use crate::awareness::Awareness;
use crate::manifest::Manifest;
use std::fmt;
use std::path::PathBuf;

/// 埋め込みに失敗した理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// ターゲットがWindowsではない(埋め込み不要)。
    NotWindows,
    /// MSVC以外のWindowsツールチェーン(未対応)。
    UnsupportedToolchain(String),
    /// `build.rs`の外で呼ばれた(Cargoの環境変数が無い)。
    NotInBuildScript(&'static str),
    Io(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::NotWindows => f.write_str("ターゲットがWindowsではないため、埋め込みは不要です / target is not Windows; nothing to embed"),
            BuildError::UnsupportedToolchain(t) => write!(f, "マニフェストの埋め込みはMSVCターゲットのみ対応です(ターゲットの環境: {t})。実行時の rs_dpi_awareness::init() を使ってください / manifest embedding supports MSVC targets only (target env: {t}); use rs_dpi_awareness::init() at run time"),
            BuildError::NotInBuildScript(v) => write!(f, "build.rsの中から呼んでください(環境変数 {v} がありません) / call this from build.rs (missing env var {v})"),
            BuildError::Io(e) => write!(f, "マニフェストを書き出せません: {e} / cannot write the manifest: {e}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// 一番かんたんな入口: 失敗してもビルドは止めず、必要なら警告だけ出す。
pub fn embed(awareness: Awareness) {
    embed_manifest_or_warn(&Manifest::new(awareness));
}

/// [`embed`]の、細かく設定したマニフェスト版。
pub fn embed_manifest_or_warn(manifest: &Manifest) {
    match embed_manifest(manifest) {
        Ok(_) | Err(BuildError::NotWindows) => {}
        Err(e) => println!("cargo:warning=rs-dpi-awareness: {e}"),
    }
}

/// マニフェストを`OUT_DIR`へ書き出し、リンカーに埋め込ませる指示(`cargo:`行)を標準出力へ出す。書き出したパスを返す。
pub fn embed_manifest(manifest: &Manifest) -> Result<PathBuf, BuildError> {
    let var =
        |name: &'static str| std::env::var(name).map_err(|_| BuildError::NotInBuildScript(name));
    if var("CARGO_CFG_TARGET_OS")? != "windows" {
        return Err(BuildError::NotWindows);
    }
    let env = var("CARGO_CFG_TARGET_ENV")?;
    if env != "msvc" {
        return Err(BuildError::UnsupportedToolchain(env));
    }
    let dir = PathBuf::from(var("OUT_DIR")?);
    let path = dir.join("rs-dpi-awareness.manifest");
    std::fs::write(&path, manifest.to_xml()).map_err(|e| BuildError::Io(e.to_string()))?;
    let p = path.to_string_lossy().replace('\\', "/");
    // 種類を限定した`rustc-link-arg-bins`等は、その種類のターゲットが無いとCargoがエラーにする。
    // 汎用の`rustc-link-arg`は、リンクされる全てのターゲット(bin・example・test・cdylib)に効き、無い種類があっても害がない。
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{p}");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_a_build_script_it_says_so_instead_of_panicking() {
        // テストの実行中は`CARGO_CFG_TARGET_OS`が無い。
        if std::env::var("CARGO_CFG_TARGET_OS").is_err() {
            assert_eq!(
                embed_manifest(&Manifest::default()),
                Err(BuildError::NotInBuildScript("CARGO_CFG_TARGET_OS"))
            );
        }
    }

    #[test]
    fn error_messages_are_bilingual() {
        let m = BuildError::UnsupportedToolchain("gnu".into()).to_string();
        assert!(m.contains("MSVC") && m.contains("init()"));
        assert!(m.contains("supports MSVC targets only"));
    }
}
