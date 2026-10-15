# PORTING(rs-dpiAwareness)

English: [PORTING/PORTING.en.md](PORTING/PORTING.en.md)

## 現状(2026-10-16、v0.1.0)

実装済み・実機(Windows 11、MSVC、4K・200%のモニター1枚)で確認済み:
- `init()`/`set()`/`process_awareness()`(段階的な代替・再取得・冪等)、モニター列挙、`WM_DPICHANGED`、DPI別のウィンドウ寸法、換算、
  マニフェストの生成/解析/EXEからの読み出し、`build.rs`での埋め込み(埋め込みだけでPerMonitorV2になることをEXEで確認)、診断CLI、実ウィンドウのサンプル。
- テスト: ユニット26 + 実機/結合(`windows_apply`・`windows_runtime`・`windows_window`・`embed_manifest`)。

## 未了・次にやること

1. **DPIの違う複数モニター間の移動**の実機確認(今は合成`WM_DPICHANGED`のみ)。
2. **Windows 10 1703未満**での代替(PerMonitor/System)の実機確認。
3. `*-pc-windows-gnu`でのマニフェスト埋め込み(`windres`経由)。今は警告を出すだけ。
4. EXEのマニフェスト読み出しを、純Rustの簡易PE解析にしてWindows以外でも`check`を使えるようにする。
5. crates.ioへの公開(名前`rs-dpi-awareness`が空いているか確認してから)。
6. CIの結果確認(GitHub Actionsのwindowsランナーは画面が無い場合があり、モニター列挙のテストが失敗するなら条件を緩める)。

## 設計の判断(理由つき)

- **新しいAPIは実行時に解決**: 静的リンクだと古いWindowsでEXEが起動しない。
- **結果は再取得して返す**: 「設定したつもり」を報告しない。既に決まっていた場合は`AlreadySet`と正直に返す。
- **`windows_apply.rs`のテストは1つだけ**: DPI認識は一度決まると変えられないため、同一プロセス内の複数テストは順序依存になる。
- **`cargo:rustc-link-arg`(汎用)を使う**: `-bins`/`-examples`/`-tests`を個別に出すと、その種類が無いパッケージでCargoがエラーにする(実際にfixtureのビルドで起きた)。
- **RPoemは使わない**: OSのAPIを直接呼ぶ低レベルの部品で、Webフレームワークの出番が無い。
