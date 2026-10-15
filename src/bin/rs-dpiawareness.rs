//! rs-dpiawareness — DPI認識の診断・マニフェスト生成・換算のCLI。

use rs_dpi_awareness::{scale, Awareness, Declared, Manifest, ManifestError};
use std::process::ExitCode;

const HELP: &str = "\
rs-dpiawareness — Windows DPI awareness inspector / Windows DPI認識の診断ツール

  rs-dpiawareness info                    このPCのDPI認識とモニターを表示 / show awareness and monitors
  rs-dpiawareness check <exe>             EXEのマニフェストがDPI認識を宣言しているか調べる / check an EXE's declaration
  rs-dpiawareness manifest [options]      マニフェストを標準出力へ / print an app manifest
      --awareness <v2|pm|system|unaware|gdi>   (既定 v2)
      --common-controls   --long-path   --utf8   --no-win10
  rs-dpiawareness scale <px> <dpi>        論理px→物理px / logical → physical pixels   (例: scale 100 144 → 150)
  rs-dpiawareness unscale <px> <dpi>      物理px→論理px / physical → logical pixels
  rs-dpiawareness help
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print!("{HELP}");
            Ok(())
        }
        Some("info") => info(),
        Some("check") => check(args.get(1)),
        Some("manifest") => manifest(&args[1..]),
        Some("scale") => convert(&args[1..], false),
        Some("unscale") => convert(&args[1..], true),
        Some(other) => Err(format!("不明なコマンドです: {other}\n{HELP}")),
    };
    match code {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn info() -> Result<(), String> {
    println!(
        "DPI対応: {} / supported: {}",
        if rs_dpi_awareness::is_supported() {
            "あり"
        } else {
            "なし(Windows以外)"
        },
        rs_dpi_awareness::is_supported()
    );
    println!(
        "起動時の認識 / awareness at launch: {}",
        rs_dpi_awareness::process_awareness()
    );
    let o = rs_dpi_awareness::init();
    println!("{o}");
    println!(
        "システムDPI / system DPI: {} ({}%)",
        rs_dpi_awareness::system_dpi(),
        scale::percent(rs_dpi_awareness::system_dpi())
    );
    let ms = rs_dpi_awareness::monitors();
    if ms.is_empty() {
        println!("モニター: 取得できません / monitors: none found");
    }
    for (i, m) in ms.iter().enumerate() {
        println!(
            "モニター{} {}{}: {}x{} @({},{})  DPI {}x{} = {}%  作業領域 {}x{} / monitor {}",
            i + 1,
            m.device_name,
            if m.primary {
                " [メイン/primary]"
            } else {
                ""
            },
            m.rect.width(),
            m.rect.height(),
            m.rect.left,
            m.rect.top,
            m.dpi_x,
            m.dpi_y,
            m.percent(),
            m.work_rect.width(),
            m.work_rect.height(),
            i + 1
        );
    }
    if !o.is_satisfied() {
        println!("注意: PerMonitorV2にできませんでした(Windows 10 1703未満など)。 / Note: PerMonitorV2 was not available (older than Windows 10 1703?).");
    }
    Ok(())
}

fn check(path: Option<&String>) -> Result<(), String> {
    let path = path.ok_or("EXEのパスを指定してください / give the path of an EXE")?;
    match rs_dpi_awareness::read_embedded_manifest(path) {
        Ok(Some(xml)) => {
            let d = Declared::parse(&xml);
            println!("マニフェスト: 埋め込みあり / manifest: embedded");
            println!(
                "  dpiAware     = {}",
                d.dpi_aware.as_deref().unwrap_or("(なし/none)")
            );
            println!(
                "  dpiAwareness = {}",
                if d.dpi_awareness.is_empty() {
                    "(なし/none)".to_string()
                } else {
                    d.dpi_awareness.join(",")
                }
            );
            match d.effective() {
                Some(a) => println!("  → Windows 10 1607以降での認識: {a} / effective on Windows 10 1607+: {a}"),
                None => println!("  → 宣言なし。DPI非対応として扱われ、高DPIの画面でぼやけます。 / No declaration: treated as DPI-unaware (blurry on high-DPI screens)."),
            }
            if let Some(l) = d.legacy() {
                println!("  → 古いWindows(8.1〜10 1511)では: {l} / on older Windows: {l}");
            }
            if d.effective() != Some(Awareness::PerMonitorV2) {
                println!("  ヒント: PerMonitorV2にするには `rs-dpiawareness manifest` の出力を埋め込むか、起動時に rs_dpi_awareness::init() を呼んでください。");
                println!("  Hint: embed the output of `rs-dpiawareness manifest`, or call rs_dpi_awareness::init() at startup.");
            }
            Ok(())
        }
        Ok(None) => {
            println!("マニフェスト: 埋め込みなし / manifest: not embedded");
            let side = format!("{path}.manifest");
            if std::path::Path::new(&side).is_file() {
                println!("  同じ場所に {side} があります(外部マニフェストとして使われることがあります)。 / a side-by-side manifest exists");
            }
            println!("  → 宣言なし。実行時に init() を呼ばない限り、DPI非対応として扱われます。 / No declaration: DPI-unaware unless the program calls init().");
            Ok(())
        }
        Err(ManifestError::Unsupported) => {
            Err("このOSでは読み出せません(Windowsのみ) / Windows only".to_string())
        }
        Err(e) => Err(format!("{path}: {e}")),
    }
}

fn manifest(opts: &[String]) -> Result<(), String> {
    let mut m = Manifest::default();
    let mut it = opts.iter();
    while let Some(o) = it.next() {
        match o.as_str() {
            "--awareness" => {
                let v = it
                    .next()
                    .ok_or("--awareness の値がありません / --awareness needs a value")?;
                m.awareness = v.parse()?;
            }
            "--common-controls" => m.common_controls_v6 = true,
            "--long-path" => m.long_path_aware = true,
            "--utf8" => m.utf8_code_page = true,
            "--no-win10" => m.windows10_compat = false,
            other => {
                return Err(format!(
                    "不明なオプションです: {other} / unknown option: {other}"
                ))
            }
        }
    }
    print!("{}", m.to_xml());
    Ok(())
}

fn convert(args: &[String], inverse: bool) -> Result<(), String> {
    let (px, dpi) = match args {
        [a, b] => (
            a.parse::<i32>()
                .map_err(|_| format!("整数ではありません: {a}"))?,
            b.parse::<u32>()
                .map_err(|_| format!("整数ではありません: {b}"))?,
        ),
        _ => return Err("使い方: scale <px> <dpi> / usage: scale <px> <dpi>".to_string()),
    };
    let out = if inverse {
        scale::unpx(px, dpi)
    } else {
        scale::px(px, dpi)
    };
    println!("{out}");
    Ok(())
}
