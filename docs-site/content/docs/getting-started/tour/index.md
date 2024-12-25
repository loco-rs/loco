+++
title = "クイックツアー"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++


<img style="width:100%; max-width:640px" src="tour.png"/>
<br/>
<br/>
<br/>
Locoで数分でブログバックエンドを作成しましょう。まず、`loco`と`sea-orm-cli`をインストールします：

<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # DBが必要な場合のみ
```
<!-- </snip> -->


次に、新しいアプリを作成できます（「`SaaS`アプリ」を選択）。クライアントサイドレンダリングのSaaSアプリを選択します：

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ アプリ名は？ · myapp
✔ ❯ 何を作成しますか？ · クライアントサイドレンダリングのSaaSアプリ
✔ ❯ DBプロバイダーを選択 · Sqlite
✔ ❯ バックグラウンドワーカータイプを選択 · Async (プロセス内のtokio非同期タスク)

🚂 Locoアプリが正常に生成されました：
myapp/

- assets: あなたは`clientside`をアセットサービング設定として選択しました。

次のステップは、フロントエンドをビルドすることです：
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

あなたは以下を持つことになります：

* `sqlite`をデータベースとして使用します。データベースプロバイダーについては、_models_セクションの[Sqlite vs Postgres](@/docs/the-app/models.md#sqlite-vs-postgres)を参照してください。
* `async`をバックグラウンドワーカーとして使用します。ワーカーの設定については、_workers_セクションの[async vs queue](@/docs/processing/workers.md#async-vs-queue)を参照してください。
* クライアントサイドアセットのサービング設定。これは、バックエンドがAPIとして機能し、静的なクライアントサイドコンテンツも提供することを意味します。

次に、`myapp`に`cd`して、`cargo loco start`を実行してアプリを開始します：
 
 <div class="infobox">
 クライアントサイドアセットサービングオプションが設定されている場合、サーバーを開始する前にフロントエンドをビルドすることを確認してください。これは、フロントエンドディレクトリに移動（`cd frontend`）し、`pnpm install`と`pnpm build`を実行することで行えます。
 </div>

<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
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
<!-- </snip> -->


<div class="infobox">
`cargo`を通して実行する必要はありませんが、開発中は強く推奨されます。`--release`をビルドすると、バイナリにはコードや`cargo`、Rustが含まれます。
</div>

## CRUD APIの追加

私たちはユーザー認証が生成された基本的なSaaSアプリを持っています。`post`を追加して、完全なCRUD APIを`scaffold`を使用して作成しましょう：

<div class="infobox">
それぞれの`-api`、`--html`、および`--htmx`フラグを使用して、`api`、`html`、または`htmx`のスキャフォールドを生成できます。
</div>

クライアント向けのクライアントサイドコードベースを持つバックエンドを構築しているので、`--api`を使用してAPIを構築します：

```sh
$ cargo loco generate scaffold post title:string content:text --api

  :
  :
added: "src/controllers/post.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/post.rs"
injected: "tests/requests/mod.rs"
* `post`のマイグレーションが追加されました！ `$ cargo loco db migrate`で適用できます。
* モデル`posts`のテストが追加されました。`cargo test`で実行します。
* コントローラー`post`が正常に追加されました。
* コントローラー`post`のテストが正常に追加されました。`cargo test`を実行します。
```

データベースがマイグレーションされ、モデル、エンティティ、および完全なCRUDコントローラーが自動的に生成されました。

再度アプリを開始します：
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
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
<!-- </snip> -->

<div class="infobox"> 
どのスキャフォールドテンプレートオプションを選択したかによって（`-api`、`--html`、`--htmx`）、スキャフォールドリソースの作成手順が変わります。`--api`フラグまたは`--htmx`フラグを使用すると、以下の例を使用できます。しかし、`--html`フラグを使用する場合は、ブラウザで投稿作成手順を行うことを推奨します。
  
`curl`を使用して`--html`スキャフォールドをテストしたい場合は、リクエストを`Content-Type: application/x-www-form-urlencoded`で送信し、ボディはデフォルトで`title=Your+Title&content=Your+Content`とする必要があります。必要に応じて、コード内で`Content-Type`を`application/json`に変更できます。
</div>

次に、`curl`を使用して`post`を追加してみましょう：

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:5150/api/posts
```

投稿をリストできます：

```sh
$ curl localhost:5150/api/posts
```

ブログバックエンドを作成するためのコマンドは次のとおりです：

1. `cargo install loco`
2. `cargo install sea-orm-cli`
3. `loco new`
4. `cargo loco generate scaffold post title:string content:text --api`

完了です！`loco`と一緒に楽しんでください 🚂

## SaaS認証の確認

生成されたアプリには、JWTに基づく完全に機能する認証スイートが含まれています。

### 新しいユーザーの登録

`/api/auth/register`エンドポイントは、アカウント確認のための`email_verification_token`を持つ新しいユーザーをデータベースに作成します。確認リンク付きのウェルカムメールがユーザーに送信されます。

```sh
$ curl --location 'localhost:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

セキュリティ上の理由から、ユーザーがすでに登録されている場合、新しいユーザーは作成されず、ユーザーのメール詳細を公開せずに200ステータスが返されます。

### ログイン

新しいユーザーを登録した後、次のリクエストを使用してログインします：

```sh
$ curl --location 'localhost:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

レスポンスには、認証用のJWTトークン、ユーザーID、名前、および確認ステータスが含まれます。

```sh
{
    "token": "...",
    "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
    "name": "Loco user",
    "claims": null,
    "is_verified": false
}
```

クライアントサイドアプリでは、このJWTトークンを保存し、次のリクエストを行う際に_ベアラートークン_（下記参照）を使用して認証を行います。

### 現在のユーザーを取得

このエンドポイントは認証ミドルウェアによって保護されています。以前に取得したトークンを使用して、_ベアラートークン_技術でリクエストを実行します（`TOKEN`を以前に取得したJWTトークンに置き換えます）：

```sh
$ curl --location --request GET 'localhost:5150/api/auth/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN'
```

これが最初の認証リクエストになります！

`controllers/auth.rs`のソースコードを確認して、自分のコントローラーで認証ミドルウェアをどのように使用するかを確認してください。
