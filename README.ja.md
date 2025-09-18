<div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Locoへようこそ</h1>

   <h3>
🚂 LocoはRust on Railsです。
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

English · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ 日本語 · [한국어](./README.ko.md) · [Русский](./README.ru.md)

## Locoとは？
`Loco`はRailsに強くインスパイアされています。RailsとRustの両方を知っているなら、すぐに馴染むでしょう。Railsしか知らなく、Rustに新しい方でも、Locoは新鮮に感じるでしょう。Railsを知っているとは仮定していません。

Locoの動作についての詳細なガイド、例、APIリファレンスは、[ドキュメント](https://loco.rs)をチェックしてください。

## Locoの特徴：

* `設定より規約:` Ruby on Railsに似て、Locoはボイラープレートコードを減らすことでシンプルさと生産性を発揮します。合理的なデフォルトを使用し、開発者が設定に時間を費やすのではなく、ビジネスロジックの記述に集中できるようにします。

* `迅速な開発:` 高い開発者生産性を目指し、Locoの設計はボイラープレートコードを減らし、直感的なAPIを提供することに焦点を当てています。これにより、開発者は迅速に反復し、最小限の努力でプロトタイプを構築できます。

* `ORM統合:` ビジネスモデルを堅牢なエンティティで表現し、SQLを書く必要をなくします。エンティティに直接関係、検証、およびカスタムロジックを定義でき、メンテナンス性とスケーラビリティが向上します。

* `コントローラー:` ウェブリクエストのパラメータ、ボディ、検証を処理し、コンテンツに応じたレスポンスをレンダリングします。最高のパフォーマンス、シンプルさ、拡張性のためにAxumを使用しています。コントローラーは、認証、ロギング、エラーハンドリングなどのロジックを追加するためのミドルウェアを簡単に構築できます。

* `ビュー:` Locoはテンプレートエンジンと統合し、テンプレートから動的なHTMLコンテンツを生成できます。

* `バックグラウンドジョブ:` Redisバックエンドキューやスレッドを使用して、計算またはI/O集約型のジョブをバックグラウンドで実行します。ワーカーを実装するのは、Workerトレイトのperform関数を実装するだけです。

* `スケジューラー:` 従来の、しばしば面倒なcrontabシステムを簡素化し、タスクやシェルスクリプトをスケジュールするのをより簡単かつエレガントにします。

* `メール送信:` メール送信者は、既存のLocoバックグラウンドワーカーインフラストラクチャを使用して、バックグラウンドでメールを配信します。すべてがシームレスに行われます。

* `ストレージ:` Locoのストレージでは、ファイル操作を簡素化します。ストレージはメモリ内、ディスク上、またはAWS S3、GCP、Azureなどのクラウドサービスを使用できます。

* `キャッシュ:` Locoは、頻繁にアクセスされるデータを保存することでアプリケーションのパフォーマンスを向上させるためのキャッシュレイヤーを提供します。

Locoの詳細な機能については、[ドキュメントウェブサイト](https://loco.rs/docs/getting-started/tour/)を確認してください。

## 始め方
```sh
cargo install loco
cargo install sea-orm-cli # データベースが必要な場合のみ
```

以下で新しいアプリを作成できます（「`SaaS`アプリ」を選択）。

```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)
✔ ❯ Select an asset serving configuration · Client (configures assets for frontend serving)

🚂 Loco app generated successfully in:
myapp/
```

次に`myapp`に移動し、アプリを起動します：
```sh
$ cargo loco start

                      ▄     ▀
                                ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
██████  █████   ███ █████   ███ █████   ███ ▀█
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
██████  █████   ███ █████       █████   ███ ████▄
██████  █████   ███ █████   ▄▄▄ █████   ███ █████
██████  █████   ███  ████   ███ █████   ███ ████▀
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

listening on port 5150
```

## GUI コマンドコンソール

グラフビューアには、承認済みの `cargo loco` コマンドをブラウザから実行できるコンソールが追加されました。
`cargo run` など `debug_assertions` が有効なビルドでは自動的に利用可能です。プロダクションビルドで利用する場合は、
ブート時に [`CliAutomationService`](./src/controller/cli_console.rs) を `AppContext` に登録してください。【F:src/controller/cli_console.rs†L83-L104】【F:src/boot.rs†L510-L535】
ドクター画面のアシスタント機能を有効化するには `introspection_assistant` フィーチャーでコンパイルし、`__/loco/assistant` を公開する必要があります。【F:src/controller/monitoring.rs†L59-L103】

これらのエンドポイントはローカルツールチェーン（`cargo loco`、各種ジェネレーター、タスク）を呼び出します。信頼できるネットワークだけに公開し、
リバースプロキシで認証を挟むか、必要に応じて本番環境では無効化してください。`debug_assertions` が無効な場合はアダプターが登録されず、
エンドポイントは `404` を返すため、ホスティング環境で意図せずコマンド実行が露出することを防げます。【F:src/boot.rs†L510-L535】【F:src/controller/cli_console.rs†L173-L182】

セットアップ手順とハードニングの詳細は [`docs-site`](./docs-site/content/docs/extras/gui-console.md) を参照してください。

## Locoによって開発されています
+ [SpectralOps](https://spectralops.io) - Locoフレームワークによる各種サービス
+ [Nativish](https://nativi.sh) - Locoフレームワークによるアプリバックエンド

## 貢献者 ✨
これらの素晴らしい人々に感謝します：

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
