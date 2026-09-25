---
title: Додаємо фоновий воркер
description: Створіть воркер генератором, реалізуйте BackgroundWorker, поставте завдання в чергу через perform_later і зареєструйте його в connect_workers.
sidebar:
  order: 20
---

Мета: винести повільну або некритичну для обробки запиту роботу (надсилання звіту, виклик стороннього API, зміну розміру зображення) з потоку обробки запиту у фонове завдання.

## Передумови

- Налаштований бекенд черги (Redis, Postgres або SQLite), якщо ви хочете, щоб завдання переживали перезапуск. Якщо ви ще не визначилися, дивіться [Вибір бекенда черги](/uk/docs/how-to/choose-queue-backend/). Для локальної розробки цей крок можна пропустити — типовий режим `BackgroundQueue` без конфігурації `queue:` все одно працює, просто не зберігає завдання (якщо провайдер не налаштовано, завдання відкидаються із записом помилки в лог). Багато застосунків стартують із `workers.mode: BackgroundAsync`, який взагалі не потребує бекенда черги.

## 1. Створюємо воркер

```sh
cargo loco generate worker report_worker
```

Це створить `src/workers/report_worker.rs`, додасть `pub mod report_worker;` до `src/workers/mod.rs` і впише виклик реєстрації в `connect_workers` у `src/app.rs`. Також буде згенеровано заготовку тесту в `tests/workers/`.

Згенерована структура завжди називається `Worker` (в області видимості власного модуля `workers::report_worker`) і має порожню структуру `WorkerArgs`, яку ви заповните:

```rust
use serde::{Deserialize, Serialize};
use loco_rs::prelude::*;

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "ReportWorker".to_string()
    }

    async fn perform(&self, _args: WorkerArgs) -> Result<()> {
        // TODO: тут розміщується логіка завдання
        Ok(())
    }
}
```

## 2. Додаємо типізовані аргументи та логіку завдання

Заповніть `WorkerArgs` даними, які потрібні завданню (вони серіалізуються в чергу, тож тримайте їх компактними і `Serialize + Deserialize`), а потім реалізуйте `perform`:

```rust
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

pub struct DownloadWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    pub user_guid: String,
}

#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {
        // .. виконуємо реальну роботу, self.ctx дає доступ до БД/кешу/іншого ..
        println!("processing download for {}", args.user_guid);
        Ok(())
    }
}
```

(Цей приклад повторює `examples/demo/src/workers/downloader.rs`.)

## 3. Перевіряємо реєстрацію

Генератор уже вписав це за вас, але варто розуміти, що саме він зробив — `Hooks::connect_workers` — це місце, де кожен воркер реєструється в спільній черзі `Queue`:

```rust
// src/app.rs
#[async_trait]
impl Hooks for App {
    // ..
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }
    // ..
}
```

Якщо ви написали воркер вручну, а не згенерували його, додайте рядок `queue.register(...)` самостійно.

## 4. Ставимо завдання в чергу

Викличте метод трейту `perform_later` із контролера, таска або іншого воркера:

```rust
DownloadWorker::perform_later(
    &ctx,
    DownloadWorkerArgs {
        user_guid: "foo".to_string(),
    },
)
.await?;
```

`perform_later` повертає `Result<String>` — ідентифікатор завдання, а не `Result<()>`. У режимі `BackgroundQueue` ідентифікатор призначає провайдер черги; у режимах `ForegroundBlocking`/`BackgroundAsync` Loco генерує свіжий UUID, тож ви завжди отримуєте стабільний дескриптор:

```rust
let job_id: String = DownloadWorker::perform_later(&ctx, args).await?;
```

Якщо `workers.mode` — це `BackgroundQueue`, але провайдер черги недоступний, `perform_later` поверне `Error::QueueProviderMissing`, і завдання не виконається. Конфігурація `BackgroundQueue` без секції `queue:` падає на старті, тож зазвичай ви побачите це при запуску, а не в місці виклику.

Якщо для цього конкретного завдання потрібен вищий/нижчий пріоритет, використайте натомість `perform_later_with_priority` — семантику пріоритетів, спільну для всіх трьох бекендів, дивіться в [Вибір бекенда черги](/uk/docs/how-to/choose-queue-backend/#черги-з-пріоритетами):

```rust
DownloadWorker::perform_later_with_priority(&ctx, args, Some(50)).await?;
```

### Ставимо багато завдань в чергу одночасно

Коли одна дія розгалужується на багато завдань — сповіщення кожному учаснику команди, імпорт, що створює одне завдання на рядок — використайте `perform_all_later` замість виклику `perform_later` у циклі. Він приймає `Vec` аргументів і ставить у чергу весь пакет за один обмін із чергою, як це робить `ActiveJob.perform_all_later` у Rails:

```rust
let args_list: Vec<DownloadWorkerArgs> = team
    .members
    .iter()
    .map(|member| DownloadWorkerArgs { user_guid: member.guid.clone() })
    .collect();

let job_ids: Vec<String> = DownloadWorker::perform_all_later(&ctx, args_list).await?;
```

Він повертає один ідентифікатор завдання на кожен аргумент, у тому самому порядку. Пакет атомарний на всіх трьох вбудованих бекендах: або всі завдання потрапляють у чергу, або жодне, тож невдалий виклик можна безпечно повторити без дублювання завдань. Порожній `Vec` — це no-op.

Щоб задати кожному завданню власний пріоритет, об'єднайте кожен аргумент з `Option<i32>` і викличте `perform_all_later_with_priority` (`None` означає пріоритет за замовчуванням):

```rust
DownloadWorker::perform_all_later_with_priority(
    &ctx,
    vec![(urgent_args, Some(100)), (normal_args, None)],
)
.await?;
```

`queue()` і `tags()` воркера застосовуються до кожного завдання в пакеті, так само як і для `perform_later`. У режимі `ForegroundBlocking` завдання виконуються одне за одним у порядку введення, і перша помилка перериває виконання; у режимі `BackgroundAsync` на кожне завдання створюється окремий таск. Пріоритет у цих двох режимах не діє.

## 5. Запускаємо процес воркера

Спосіб запуску воркера залежить від `workers.mode` (дивіться [Вибір бекенда черги](/uk/docs/how-to/choose-queue-backend/):

```sh
# Режим BackgroundQueue: запустити окремий процес воркера
cargo loco start --worker

# або запустити сервер + воркер в одному процесі
cargo loco start --server-and-worker
```

Режими `BackgroundAsync` і `ForegroundBlocking` не потребують окремого процесу воркера — завдання виконуються всередині того процесу, який викликав `perform_later`.

### Фільтрування за тегами

Дайте воркеру теги, а потім запустіть процес воркера, який братиме лише відповідні завдання:

```rust
fn tags() -> Vec<String> {
    vec!["download".to_string(), "network".to_string()]
}
```

```sh
cargo loco start --worker download,network
```

Воркер, запущений без тегів (`cargo loco start --worker`), обробляє лише завдання без тегів; `--all` і `--server-and-worker` не підтримують фільтрування за тегами.

## 6. Перевіряємо

Тестуйте з режимом `ForegroundBlocking` у `config/test.yaml`, щоб `perform_later` виконувався синхронно і повертався лише після завершення завдання:

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn test_run_download_worker() {
    let boot = boot_test::<App>().await.unwrap();

    assert!(
        DownloadWorker::perform_later(
            &boot.app_context,
            DownloadWorkerArgs { user_guid: "foo".to_string() }
        )
        .await
        .is_ok()
    );

    // .. тут перевіряйте побічні ефекти ..
}
```

Тести воркерів кладіть у `tests/workers/` — генератор робить це за вас автоматично.

## Довідка

- Усі ключі YAML `queue:`/`workers:`: [Довідник конфігурації](/uk/docs/reference/configuration/#queue)
- Прапорці `cargo loco start`/`jobs`: [Довідник CLI](/uk/docs/reference/cli/)
- Feature-прапорці `worker`/`worker_redis`: [Довідник feature-прапорців](/uk/docs/reference/feature-flags/)
