# 開発方針＆開発環境ルール(rs-dpiAwareness)

全リポジトリ共通の開発ルール(自動継続・実機での検証徹底・UI/インストーラーの品質ルール等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`を正本として参照すること。
この文書には本リポジトリ固有の事項だけを書く。 / English: [CLAUDE/CLAUDE.en.md](CLAUDE/CLAUDE.en.md)

## 役割

WindowsのDPI認識(PerMonitorV2)を扱う、依存ゼロのRustクレート+診断CLI(`rs-dpiawareness`)。
`rs-FFmpeg`・`rs-xorriso`と同じ「公開仕様への敬意を込めたRust版」の位置づけで、Microsoftとは無関係。
コードは公開ドキュメントだけを根拠に書く(Microsoftやwinit等のコードを写さない)。

## 構成

- `src/awareness.rs` — 認識レベル(`Awareness`)、`init()`/`set()`/`process_awareness()`、段階的な代替、結果の再取得
- `src/sys.rs` — Win32 APIの呼び出し層。**新しいAPIは`GetProcAddress`で実行時に探す**(静的リンクにすると古いWindowsで起動しない)
- `src/monitor.rs` — モニター列挙・DPI・`WM_DPICHANGED`(`DpiChanged`)・DPI別のウィンドウ寸法
- `src/scale.rs` — 論理⇔物理ピクセルの換算(`MulDiv`と同じ丸め)
- `src/manifest.rs` — マニフェストの生成(`Manifest`)・解析(`Declared`)・EXEからの読み出し
- `src/build.rs` — `build.rs`から使う埋め込み補助(MSVCのみ。汎用の`cargo:rustc-link-arg`を使う: 種類限定の`-bins`等は、その種類が無いパッケージでCargoがエラーにする)
- `src/bin/rs-dpiawareness.rs` — 診断CLI
- `examples/window.rs` — 実ウィンドウのサンプル。`--selftest`で合成`WM_DPICHANGED`に追従するか自己診断
- `tests/` — 実機テスト。**`windows_apply.rs`はテストを1つだけにしている**(DPI認識は一度決まると変えられず、同じプロセス内の複数テストは実行順で結果が変わるため)。`embed_manifest.rs`は実際にEXEをビルドして確かめる

## ルール

- 依存クレートは足さない(依存ゼロが売り)。
- Windows APIを足すときは、まず`GetProcAddress`で解決できるか考える。
- 「確かめたこと/確かめていないこと」をREADMEに正直に書く。複数モニター間の移動、Windows 10 1703未満、GNUターゲットは未確認。
- 検証は実機(Windows)で。CIの成功だけで完了としない。
- コミット前に `cargo fmt` / `cargo clippy --all-targets -- -D warnings` / `cargo test -- --test-threads=1`。
