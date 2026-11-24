> Latest translations / 翻訳の最新版: [0-LATEST-TRANSLATIONS.md](0-LATEST-TRANSLATIONS.md)

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

---

作者 / Author: 石 塚 正 浩（MASAHIRO ISHIZUKA） (https://github.com/aon-co-jp)

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**
