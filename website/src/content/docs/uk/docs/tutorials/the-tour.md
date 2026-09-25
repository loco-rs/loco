---
title: Тур
description: "Швидший наскрізний огляд: модель зі зв'язком, маршрут контролера, зібраний вручну, фоновий воркер і завдання — за один присідання."
sidebar:
  order: 2
---

Цей тур рухається швидше, ніж [Ваш перший застосунок](/uk/docs/tutorials/your-first-app/), і припускає, що ви вже встановили `loco` та `sea-orm-cli` і згенерували принаймні один застосунок. За один прохід ви торкнетеся чотирьох складових, з яких складається майже кожна функція Loco: **моделі**, **контролери**, **воркери** та **завдання**. Кожен розділ посилається на довідкову сторінку, яка документує його повну поверхню — ця сторінка показує лише робочий шлях.

## Підготовка

```sh
loco new --name tour_app --db sqlite --bg async --assets none
cd tour_app
```

<div class="infobox">
Вибір бази даних (<code>--db sqlite</code>) також дає цьому застосунку готовий набір автентифікації на <code>/api/auth/*</code> — він розглядається окремо в <a href="/uk/docs/tutorials/saas-with-auth/">Створіть невеликий застосунок з автентифікацією</a>. Цей тур його ігнорує та будує власні ресурси поруч із ним.
</div>

## Моделі та контролери: скафолд і проста модель зі зв'язком

Згенеруйте скафолд `posts` — модель, міграцію, сутність, CRUD-контролер і тести за один крок (без автентифікації):

```sh
$ cargo loco generate scaffold posts title:string! content:text --no-auth
```

Тепер згенеруйте **лише модель** `comments` (без контролера — його ви напишете вручну), яка належить до поста, використовуючи тип поля `references`:

```sh
$ cargo loco generate model comments content:text post:references
```

Обидві команди записують міграцію, **застосовують її негайно** та перегенерують сутності Sea-ORM — окремого кроку «тепер запустіть міграцію» немає. Відкрийте міграцію comments, і ви побачите зв'язок як простий опис даних, а не рукописні виклики SQL-білдера:

```rust
// migration/src/mYYYYMMDD_HHMMSS_comments.rs
use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(m, "comments",
            &[
            ("id", ColType::PkAuto),
            ("content", ColType::TextNull),
            ],
            &[
            ("post", ""),
            ]
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "comments").await
    }
}
```

`("post", "")` означає «додати зовнішній ключ до `posts` і самостійно визначити назву колонки» — вона стає обов'язковою колонкою `post_id`. Ця колонка є 64-бітним цілим числом (`BigInteger`), що відповідає 64-бітним автоінкрементним первинним ключам, які Loco 1.0 використовує всюди; дивіться [Схема та DSL ColType](/uk/docs/reference/schema-dsl/) для повної картини типів колонок.

## Контролери: згенерований і зібраний вручну

`posts` уже має повний CRUD-контролер від свого скафолда. У `comments` поки що є лише модель — `scaffold` генерує модель *й* контролер разом, тоді як `model` будує лише шар даних. Звертайтеся до `controller`, коли модель уже є, а потрібно лише тонке API над нею:

```sh
$ cargo loco generate controller comments
```

Без указаних назв дій це створює заглушку з однією дією `index`, що повертає порожнє тіло — вона нічого не знає про колонки вашої моделі. Повністю замініть `src/controllers/comments.rs` на невелике, цілеспрямоване API: коментарі можна лише **додавати** через поверхневий маршрут і **переглядати списком** через вкладений маршрут під їхнім постом — ніколи не отримувати поодинці, не оновлювати і не видаляти:

```rust
#![allow(clippy::unused_async)]
use axum::http::StatusCode;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::_entities::comments::ActiveModel;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub content: Option<String>,
    pub post_id: i64,
}

async fn create(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let item = ActiveModel {
        content: Set(params.content),
        post_id: Set(params.post_id),
        ..Default::default()
    };
    let item = item.insert(&ctx.db).await?;

    Ok((StatusCode::CREATED, format::json(item)).into_response())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/comments/")
        .add("/", post(create))
}
```

Тепер додайте вкладене читання на боці `posts`. У `src/controllers/posts.rs` (згенерованому скафолдом) додайте маршрут і обробник, який завантажує коментарі поста через зв'язок, згенерований Sea-ORM для вас:

```rust
// add to the existing imports
use crate::models::_entities::comments;

async fn comments_for_post(
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let Some(model) = Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(not_found("post not found"));
    };
    let comments = model.find_related(comments::Entity).all(&ctx.db).await?;

    format::json(comments)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/posts")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{id}", get(get_one))
        .add("/{id}", put(update))
        .add("/{id}", delete(remove))
        .add("/{id}/comments", get(comments_for_post))
}
```

Спробуйте:

```sh
cargo loco start
```

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{"title":"Tour post","content":"hi"}' localhost:5150/api/posts
{"id":1,...}

$ curl -X POST -H "Content-Type: application/json" -d '{"content":"nice post","post_id":1}' localhost:5150/api/comments
{"id":1,...}

$ curl localhost:5150/api/posts/1/comments
[{"id":1,"content":"nice post","post_id":1,...}]
```

## Воркери: зробіть щось у фоні

Згенеруйте воркера та подивіться, як він самореєструється у `src/app.rs`:

```sh
$ cargo loco generate worker notifier
```

```
added: "src/workers/notifier.rs"
injected: "src/workers/mod.rs"
injected: "src/app.rs"
```

Згенерована структура завжди має просту назву `Worker`, у просторі імен власного модуля (`workers::notifier::Worker`) — саме так ви будете до неї звертатися. Відредагуйте `WorkerArgs` та `perform` у `src/workers/notifier.rs`:

```rust
#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
    pub post_id: i64,
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, args: WorkerArgs) -> Result<()> {
        println!("new comment/notification for post {}", args.post_id);
        Ok(())
    }
}
```

Тепер поставте його в чергу з `posts::add` у `src/controllers/posts.rs`, одразу після вставки поста:

```rust
#[debug_handler]
async fn create(State(ctx): State<AppContext>, Json(params): Json<CreatePost>) -> Result<Response> {
    let item = ActiveModel {
        title: Set(params.title),
        content: Set(params.content),
        ..Default::default()
    };
    let item = item.insert(&ctx.db).await?;

    crate::workers::notifier::Worker::perform_later(
        &ctx,
        crate::workers::notifier::WorkerArgs { post_id: item.id },
    )
    .await?;

    Ok((StatusCode::CREATED, format::json(item)).into_response())
}
```

Оскільки ви згенерували цей застосунок з `--bg async`, `workers.mode` у `config/development.yaml` є `BackgroundAsync` — завдання виконується у тому самому процесі, окремий процес `--worker` не потрібен. Перезапустіть з `cargo loco start`, зробіть `POST` нового поста та спостерігайте, як `println!` воркера з'являється в тому самому терміналі.

Асинхронність у процесі — це зручність для розробки. Для справжнього розгортання ви зазвичай перейшли б на чергу на основі Redis, Postgres або SQLite і запускали воркери як окремі процеси з `cargo loco start --worker`; дивіться [Додайте фонового воркера](/uk/docs/how-to/add-worker/) щодо компромісу async-проти-черги.

## Завдання: разове, запускається з CLI

Згенеруйте завдання:

```sh
$ cargo loco generate task posts_report
```

Відредагуйте `src/tasks/posts_report.rs`:

```rust
use loco_rs::prelude::*;

use crate::models::_entities::posts;

pub struct PostsReport;
#[async_trait]
impl Task for PostsReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "posts_report".to_string(),
            detail: "Print every post's title".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        let posts = posts::Entity::find().all(&app_context.db).await?;
        for post in &posts {
            println!("- {}", post.title);
        }
        println!("done: {} posts", posts.len());
        Ok(())
    }
}
```

Перелічіть і запустіть його:

```sh
$ cargo loco task
posts_report        [Print every post's title]

$ cargo loco task posts_report
- Tour post
done: 1 posts
```

Завдання компілюються у бінарник вашого застосунку та є чутливими до середовища (`cargo loco task posts_report -e production` виконується з конфігурацією production) — безпечніша альтернатива ad-hoc SQL до живої бази даних. Завдання також можна запускати за розкладом; дивіться [Напишіть разове завдання](/uk/docs/how-to/write-task/) та [Плануйте повторювані завдання](/uk/docs/how-to/schedule-jobs/).

## Побачте все, що ви щойно з'єднали

```sh
$ cargo loco routes
```

перелічує кожен маршрут у кожному контролері — CRUD `posts` зі скафолда, рукописні маршрути `comments`, вкладений маршрут `posts/{id}/comments`, який ви додали вручну, та набір `/api/auth/*`, що прийшов разом із базою даних.

## Далі

- [Створіть невеликий застосунок з автентифікацією](/uk/docs/tutorials/saas-with-auth/) — використайте цей вбудований набір автентифікації по-справжньому: зареєструйтеся, увійдіть і захистіть маршрут JWT.
- [Додайте модель](/uk/docs/how-to/add-model/) та [Генератори та типи полів](/uk/docs/reference/generators/) — повна мінімова типів полів, фрагмент якої ви побачили тут.
- [Схема та DSL ColType](/uk/docs/reference/schema-dsl/) — кожен тип колонки, який підтримує DSL міграцій.
- [Додайте фонового воркера](/uk/docs/how-to/add-worker/), [Напишіть разове завдання](/uk/docs/how-to/write-task/), [Плануйте повторювані завдання](/uk/docs/how-to/schedule-jobs/) — повна картина за розділами цього туру про фонові завдання та завдання.
