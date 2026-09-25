---
title: Вибір бекенда черги
description: Оберіть Redis, Postgres або SQLite для фонових завдань, налаштуйте його та оберіть режим воркера.
sidebar:
  order: 21
---

Мета: вирішити, як фонові завдання (дивіться [Додаємо фоновий воркер](/uk/docs/how-to/add-worker/) ставляться в чергу, зберігаються та обробляються, і налаштувати це.

## 1. Обираємо режим воркера

`workers.mode` керує тим, чи проходять завдання взагалі через стійку чергу:

```yaml
# config/development.yaml
workers:
  mode: BackgroundQueue # за замовчуванням. Варіанти: BackgroundQueue | ForegroundBlocking | BackgroundAsync
```

| Режим | Потрібен бекенд `queue:`? | Поведінка |
|---|---|---|
| `BackgroundQueue` (за замовчуванням) | так | Ставить завдання в чергу налаштованого провайдера; окремий процес (або потік) воркера забирає їх із черги та виконує. Переживає перезапуски. |
| `ForegroundBlocking` | ні | Виконує завдання вбудовано, блокуючи викликача до завершення. Використовується в тестах. |
| `BackgroundAsync` | ні | Запускає завдання через `tokio::spawn` у тому самому процесі. Зовнішнього сховища немає — завдання втрачаються при збої/перезапуску. |

Якщо вам потрібен лише `BackgroundAsync` або `ForegroundBlocking`, на цьому можна зупинитися — повністю пропустіть конфігурацію `queue:`.

Майстер `loco new` питає про це одразу, пропонуючи `Async`, `Queue: Redis`, `Queue: Postgres`, `Queue: SQLite` або `Blocking`; вибір одного з трьох варіантів `Queue: *` автоматично налаштовує відповідні `workers.mode: BackgroundQueue`, `queue.kind` і Cargo-фічю (`worker` для Postgres/SQLite, `worker_redis` для Redis).

## 2. Обираємо та налаштовуємо бекенд

Усі три бекенди поділяють той самий API `perform_later` і семантику пріоритетів; перемикання — це зміна конфігурації, а не коду. Задайте `queue.kind` у YAML-файлі вашого середовища:

### Redis

```yaml
queue:
  kind: Redis
  uri: "<%= get_env(name='REDIS_URL', default='redis://127.0.0.1') %>"
  dangerously_flush: false # очищає чергу на старті — лише для dev/test
  queues: [high, low] # необов'язково: іменовані черги з пріоритетами, перша = найважливіша
  num_workers: 2 # кількість одночасних обробників завдань
```

Потрібна Cargo-фічя `worker_redis`. На відміну від Postgres/SQLite, вона **не** входить у типовий набір фіч — увімкніть її явно (`worker_redis` імплікує `worker`, тож обидві вказувати не потрібно):

```toml
loco-rs = { version = "...", features = ["worker_redis"] }
```

`setup()` для Redis — це no-op: схему створювати не треба.

### Postgres

```yaml
queue:
  kind: Postgres
  uri: "<%= get_env(name='PGQ_URL', default='postgres://localhost:5432/mydb') %>"
  dangerously_flush: false
  enable_logging: false
  max_connections: 20
  min_connections: 1
  connect_timeout: 500 # мс
  idle_timeout: 500 # мс
  poll_interval_sec: 1
  num_workers: 2
```

Потрібна Cargo-фічя `worker` (увімкнена за замовчуванням — для звичайної залежності `loco-rs` додаткові `features = [...]` не потрібні). Завдання зберігаються в таблиці `pg_loco_queue`; таблиця (а для таблиць до версії 1.0 — і стовпець `priority`) створюється/мігрується автоматично на старті.

### SQLite

```yaml
queue:
  kind: Sqlite
  uri: "<%= get_env(name='SQLTQ_URL', default='sqlite://loco_development.sqlite?mode=rwc') %>"
  dangerously_flush: false
  poll_interval_sec: 1
  num_workers: 2
  # решта ключів ідентичні Postgres
```

Потрібна Cargo-фічя `worker` (увімкнена за замовчуванням) — той самий прапорець, що керує бекендом Postgres вище; обидва поділяють провайдер на основі `sqlx`, а вибір між ними відбувається під час виконання через `queue.kind`. Використовується `sqlt_loco_queue` (плюс таблиця блокувань, оскільки SQLite не має `SELECT ... FOR UPDATE SKIP LOCKED`).

Підсумок: `worker` покриває черги Postgres і SQLite (уже в типовий наборі фіч), а `worker_redis` додає до них чергу Redis. Те, який бекенд фактично працює, — це вибір під час виконання — `queue.kind: Postgres | Sqlite | Redis`, — а не feature-прапорець для конкретної бази. Повний перелік ключів (включно зі значеннями за замовчуванням) — у [Довіднику конфігурації → queue](/uk/docs/reference/configuration/#queue). Назви прапорців і те, як обрізати типовий набір фіч, — у [Довіднику feature-прапорців](/uk/docs/reference/feature-flags/).

## 3. Запускаємо процес воркера

```sh
cargo loco start --worker           # окремий процес воркера
cargo loco start --server-and-worker # сервер + воркер в одному процесі
```

Про фільтрування за тегами дивіться в [Додаємо фоновий воркер](/uk/docs/how-to/add-worker/#5-запускаємо-процес-воркера).

## Черги з пріоритетами

Усі три бекенди підтримують пріоритет для кожного завдання: більший `priority` (повноцінний `i32`) забирається з черги першим; за однакового пріоритету першим виконується завдання з ранішим `run_at`, далі — за ідентифікатором завдання. Задайте його через `perform_later_with_priority` замість `perform_later`:

```rust
DownloadWorker::perform_later_with_priority(&ctx, args, Some(100)).await?;
```

Щоб поставити багато завдань в чергу за один обмін, використайте `perform_all_later` (усі з пріоритетом за замовчуванням) або `perform_all_later_with_priority` (кожне завдання об'єднане з власним `Option<i32>`). Усі три бекенди ставлять пакет у чергу атомарно — одна транзакція на Postgres/SQLite, один блок `MULTI`/`EXEC` на Redis — тож або всі завдання потрапляють у чергу, або жодне. Дивіться [Додаємо фоновий воркер](/uk/docs/how-to/add-worker/#ставимо-багато-завдань-в-чергу-одночасно).

Redis додатково підтримує **іменовані** черги через `queue.queues` — `Worker::queue()` визначає, до якої іменованої черги потрапить завдання, а порядок у списку конфігурації задає пріоритет кожної черги (перша = найважливіша). Типові іменовані черги: `["default", "mailer"]`.

## Керування завданнями з CLI

Щойно увімкнено `worker` (Postgres/SQLite) або `worker_redis` (Redis), команда `cargo loco jobs` доступна для всіх трьох бекендів — включно з Redis, який тепер повністю підтримує адміністративні операції (скасування, очищення, повторна постановка в чергу та dump/import більше не є ексклюзивом Postgres/SQLite):

```sh
cargo loco jobs cancel --name <NAME>
cargo loco jobs tidy                 # видаляє завершені/скасовані завдання
cargo loco jobs purge --max-age 90    # видаляє старі невдалі/скасовані завдання
cargo loco jobs dump -f <folder>
cargo loco jobs import -f <file>
cargo loco jobs requeue --from-age 0  # повертає завислі завдання зі статусу "processing" у "queued"
```

Повний перелік прапорців — у [Довіднику CLI](/uk/docs/reference/cli/#23-підкоманди-jobs).

### Автоматична повторна постановка в чергу (reaper)

Ручний запуск `cargo loco jobs requeue` повертає завдання, які застрягли в `processing` після збою воркера, але за замовчуванням ніщо не робить цього автоматично. Щоб запущений процес воркера робив це періодично, увімкніть блок `reaper` під `queue:` (підтримується всіма трьома бекендами):

```yaml
queue:
  kind: Postgres
  uri: "<%= get_env(name='PGQ_URL', default='postgres://localhost:5432/mydb') %>"
  # ...
  reaper:
    age_minutes: 10 # ставить у чергу завдання, що застрягли в "processing" довше цього часу
    interval_seconds: 60 # необов'язково, за замовчуванням 60 — як часто виконувати обхід
```

Якщо `reaper` не задано (за замовчуванням), попередня поведінка не змінюється — фоновий обхід не запускається, а завдання, що застрягли в `processing`, залишаються там, доки ви самі не виконаєте `cargo loco jobs requeue`.

## Що обрати з трьох

- **Redis** — найменша затримка, іменовані черги з пріоритетами, жодної додаткової схеми. Добрий вибір за замовчуванням, якщо ви вже запускаєте Redis.
- **Postgres** — жодного додаткового компонента, якщо база вашого застосунку вже Postgres; `FOR UPDATE SKIP LOCKED` забезпечує надійну конкурентність.
- **SQLite** — нуль додаткової інфраструктури для невеликих розгортань або локальної розробки; використовує таблицю блокувань замість `SKIP LOCKED`, тому менш придатний для високої конкурентності воркерів.
