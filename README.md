# rs-dpiAwareness

**言語 / Languages**: 日本語(このページ) | [English](README/README.en.md)

WindowsのDPI認識(**PerMonitorV2**)を、依存ゼロのRustで扱う小さなクレートと診断ツール。
Microsoftの公開仕様(`SetProcessDpiAwarenessContext`、アプリケーションマニフェストの`<dpiAware>`/`<dpiAwareness>`)への
敬意を込めたRust版です。Microsoftとは無関係で、コードは公開ドキュメントだけを根拠に書いています。

## なぜ必要か

DPI認識を宣言していないアプリは、高DPIの画面でWindowsに**画面ごと拡大され、ぼやけます**。
`PerMonitorV2`(Windows 10 1703以降)にすると、モニターごとのDPIに追従し、タイトルバー・ダイアログ・コモンコントロールも自動で拡大されます。
ただし設定の仕方が分かりにくい点が多く、このクレートはそこを埋めます:

- **一度決まったら変えられない**: プロセスのDPI認識はウィンドウを作る前に決める必要があり、後からは変更できません。
- **古いWindowsで起動しなくなる罠**: `SetProcessDpiAwarenessContext`などを静的にリンクすると、その関数が無い古いWindowsでEXEが起動しません。ここでは実行時に探して呼びます。
- **マニフェストが一番確実**: 実行時の設定より、プロセス開始前に決まるマニフェストの宣言のほうが取りこぼしがありません。`build.rs`から1行で埋め込めます。
- **「効いているか」が分かりにくい**: 設定した値を**再取得して報告**し、既存のEXEが何を宣言しているかも調べられます。

## 一番かんたんな使い方

```toml
# Cargo.toml (crates.ioには未公開。当面はGit指定)
[dependencies]
rs-dpi-awareness = { git = "https://github.com/aon-co-jp/rs-dpiAwareness" }
```

```rust
fn main() {
    // ウィンドウを作る前に1回だけ。古いWindowsでも PerMonitorV2 → PerMonitor → System と段階的に代替し、何度呼んでも安全。
    let outcome = rs_dpi_awareness::init();
    println!("{outcome}");
    // ...ウィンドウ作成...
}
```

### マニフェストで宣言する(推奨)

```toml
[build-dependencies]
rs-dpi-awareness = { git = "https://github.com/aon-co-jp/rs-dpiAwareness" }
```

```rust
// build.rs
fn main() {
    rs_dpi_awareness::build::embed(rs_dpi_awareness::Awareness::PerMonitorV2);
}
```

MSVCターゲットのEXEにマニフェストが埋め込まれます(Windows以外では何もしません。`*-pc-windows-gnu`は未対応で、ビルドは失敗させず警告を出します)。
細かい設定(コモンコントロールv6、長いパス、UTF-8のコードページ)は `Manifest::new(..).common_controls_v6(true)...` と `build::embed_manifest_or_warn(&m)` で。

### ウィンドウの`WM_DPICHANGED`に追従する

[`examples/window.rs`](examples/window.rs)が、外部クレートなしのWin32で動く最小の完成例です(`cargo run --example window`)。要点は次の4行:

```rust
// ウィンドウプロシージャの中
WM_DPICHANGED => {
    if let Some(d) = unsafe { DpiChanged::from_message(wparam, lparam) } {
        let r = d.suggested; // Windowsが提案する新しい位置と大きさ
        unsafe { SetWindowPos(hwnd, 0, r.left, r.top, r.width(), r.height(), SWP_NOZORDER | SWP_NOACTIVATE) };
    }
    0
}
```

### ピクセルの換算

```rust
use rs_dpi_awareness::scale;
assert_eq!(scale::px(100, 144), 150);   // 論理100px(96dpi基準)→ 144dpi(150%)の物理150px
assert_eq!(scale::unpx(150, 144), 100); // 逆
assert_eq!(scale::percent(168), 175);
```

丸めはWindowsの`MulDiv`と同じ(最も近い整数へ、半分は0から遠い方へ)なので、Windows自身の換算と1ピクセルずれません。

## 診断ツール `rs-dpiawareness`

```
rs-dpiawareness info                  このPCのDPI認識とモニターを表示
rs-dpiawareness check <exe>           EXEのマニフェストがDPI認識を宣言しているか調べる(コードは実行しない)
rs-dpiawareness manifest [--awareness v2|pm|system|unaware|gdi] [--common-controls] [--long-path] [--utf8]
rs-dpiawareness scale <px> <dpi>      論理px→物理px      (unscale: 逆)
```

実行例(このPC、4K・200%):

```
DPI認識: PerMonitorV2(希望 PerMonitorV2、設定しました) / DPI awareness: PerMonitorV2 (requested PerMonitorV2, applied via ContextApi)
システムDPI / system DPI: 192 (200%)
モニター1 \\.\DISPLAY1 [メイン/primary]: 3840x2160 @(0,0)  DPI 192x192 = 200%  作業領域 3840x2064
```

`check`は、埋め込まれたマニフェストの`<dpiAwareness>`/`<dpiAware>`を読み、**Windows 10 1607以降が実際に採用するレベル**と、古いWindowsでのレベルを分けて表示します。

## 実際に確かめたこと / 確かめていないこと(正直な開示)

確かめたこと(Windows 11 Home 10.0.26200、MSVC、4K・200%のモニター1枚):

- `init()`が、マニフェスト無しのプロセスを`PerMonitorV2`にし、再取得して同じ値を確認。2回目は`AlreadySet`、弱いレベルを頼んでも下げない。
- `build.rs`で埋め込んだ**マニフェストだけ**で(実行時に`init()`を呼ばなくても)、実際にビルドしたEXEが`PerMonitorV2`で動く。EXEから読み出したマニフェストも一致。
- 実ウィンドウで、合成した`WM_DPICHANGED`(216dpi、提案矩形つき)に追従してサイズ・タイトルが変わる(`examples/window.rs`の`--selftest`)。
- モニターの数・位置・作業領域・DPI、200%でのウィンドウ枠とスクロールバー幅がおよそ2倍になること。

**確かめていないこと**:

- **DPIの違う複数のモニター間でウィンドウを動かす実機確認**はしていません(手元はモニター1枚)。`WM_DPICHANGED`は合成メッセージでの確認です。
- **Windows 10 1703より古いOS**での代替(`PerMonitor`/`System`)は、コードはありますが実機では未確認です。
- `*-pc-windows-gnu`でのマニフェスト埋め込みは未対応です(実行時の`init()`は使えます)。
- DPI認識は**プロセス全体**の設定です。`SetThreadDpiAwarenessContext`でスレッドごとに上書きしている場合、`process_awareness()`はそのスレッドの値を返します。
- winit・Tauri・Qtなどのフレームワークは、自分でDPI認識を設定します。それらと併用する場合は、フレームワークの設定を優先してください(このクレートは、自前のWin32/GDIアプリや、診断が目的の場合に向きます)。
- Windows以外では、すべて何もしない安全な値を返します(`monitors()`は空、`system_dpi()`は96)。
- crates.ioには未公開です。

## 関連と位置づけ

- [`winit`の`dpi`クレート](https://crates.io/crates/dpi)は、論理・物理サイズを**型**で区別する別の部品です。置き換えではなく補完の関係です。
- [RPoem](https://github.com/aon-co-jp/RPoem)は使っていません: これはOSのAPIを直接呼ぶ低レベルの部品で、Webフレームワークが入る余地がないためです。
- 仲間のRust版: [rs-FFmpeg](https://github.com/aon-co-jp/rs-FFmpeg) / [rs-xorriso](https://github.com/aon-co-jp/rs-xorriso)

ライセンス: MIT
