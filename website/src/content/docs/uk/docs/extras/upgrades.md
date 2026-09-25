---
title: Оновлення
description: ""
sidebar:
  order: 4
---

## Що робити, коли вийшла нова версія Loco?

- Створіть чисту гілку у вашому репозиторії коду.
- Оновіть версію Loco у вашому головному `Cargo.toml`
- Зверніться до [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md), щоб знайти зміни, що ламають сумісність, і рефакторинги, які вам слід виконати (якщо такі є).
- Запустіть `cargo loco doctor` у вашому проєкті, щоб перевірити, що ваш застосунок і середовище сумісні з новою версією

Як завжди, якщо щось піде не так, [відкрийте issue](https://github.com/loco-rs/loco/issues) та попросіть допомоги.

## Основні залежності Loco

Loco побудований на чудових бібліотеках. Розумно слідкувати за їхніми версіями в нових релізах Loco та за їхніми власними changelog.

Ось основні з них:

- [SeaORM](https://www.sea-ql.org/SeaORM), [CHANGELOG](https://github.com/SeaQL/sea-orm/blob/master/CHANGELOG.md)
- [Axum](https://github.com/tokio-rs/axum), [CHANGELOG](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)

## Оновлення з 1.0.x до 1.1

1.1 переносить шаблонний рушій на **Tera 2** і змінює файли конфігурації так,
що вони використовують YAML-безпечні роздільники `<%= ... %>`.

Більшості застосунків потрібна **одна зміна рядка** — дивіться нижче. Тут ніщо
не вимагає переписування ваших шаблонів, якщо вони не використовують
`{% macro %}`, `{% import %}` чи доступ до масиву через `v.0` — жодне з чого
не використовується у власних згенерованих шаблонах Loco.

### Швидкий шлях: передайте це агенту для кодування

Вставте наведене нижче в Claude Code (або будь-який агент для кодування) з
відкритим вашим проєктом. Це вся поверхня змін 1.0 → 1.1, написана так, щоб
її можна було виконати. Розділи після неї — ті самі зміни, пояснені для людей.

````text
You are upgrading a Rust web application from Loco 1.0.x to Loco 1.1.

Work through the checklist below in order. For each item: search the codebase
for the pattern, apply the change only where it actually appears, and tell me
what you changed or that the item did not apply. Do not change anything that
is not listed here. When you are done, run `cargo check`, then `cargo loco
doctor`, then the test suite.

1. Cargo.toml — REQUIRED for every app.
   Set `loco-rs` to "1.1".
   If `fluent-templates` appears, set it to:
       fluent-templates = { version = "0.15", features = ["tera"] }
   Version 0.13 pins Tera 1 and will not compile against Loco 1.1. For most
   apps this is the ONLY change needed.

2. Custom Tera filters and functions (Rust code).
   Look for `register_filter`, `register_function` and `register_tester`,
   usually in src/initializers/view_engine.rs or an `after_routes` hook.
   Tera 2 changed the signatures:
       // Tera 1
       fn f(value: &serde_json::Value, args: &HashMap<String, serde_json::Value>)
           -> tera::Result<serde_json::Value>
       // Tera 2
       fn f(value: &tera::Value, kwargs: tera::Kwargs, _: &tera::State)
           -> tera::TeraResult<tera::Value>
   Read named arguments with kwargs.get::<T>("name")? for optional ones and
   kwargs.must_get::<T>("name")? for required ones.
   `&State` cannot be constructed outside the engine, so a filter can no
   longer be unit-tested by calling it. Move the logic into a plain function
   and make the filter a thin wrapper over it, so that function stays testable.
   Do NOT touch `tera.register_function("t", FluentLoader::new(...))`. That
   keeps working unchanged.

3. View and mailer templates (assets/views/**, assets/mailers/**).
   Three Tera 2 changes. Apply each only where it actually appears:
   a. `{% macro %}` and `{% import %}` were removed in favour of components.
      Templates using them need rewriting.
   b. Array access is `v[0]`, not `v.0`.
   c. An undefined variable is now an ERROR where Tera 1 rendered it empty.
      This catches MAILER templates too, so a mail template referencing an
      optional field that is sometimes absent now fails at send time rather
      than rendering nothing. Add `| default(value="")` to those references.
   Templates that Loco generated do not use (a) or (b).

4. Scaffolded list endpoints — only if you generated a scaffold.
   The JSON envelope changed to the framework's own pagination vocabulary:
       per_page    -> page_size
       total       -> total_items
       total_pages -> NEW field, add it
   The query parameter is `page_size` as well. Update any frontend or API
   client reading those fields. If you have a Loco-generated TypeScript
   frontend, regenerate bindings/ with ts-rs.

5. Traits you implemented yourself — skip if you use only built-in drivers.
   - `QueueProvider` now requires `retry_failed`.
   - `StoreDriver` now requires `list` and `stat`.
   - `StorageStrategy` now requires `list`, `stat` and `exists`.
   Every built-in driver and strategy already implements these.

6. Exhaustive `match` expressions that will stop compiling.
   - `loco_rs::doctor::Resource` is now #[non_exhaustive] and gained a
     `ProductionSafety` variant. It sorts FIRST, so the doctor report's
     display order shifts by one; code sorting or comparing Resource sees it.
   - `loco_gen::DeploymentKind` gained a `Lambda` variant.
   - `loco_gen::Component::Scaffold` and ::Controller gained a required
     `auth: bool` field.
   - `loco_gen::AppInfo` gained a `working_dir` field — pass ".".into().
   The loco_gen items matter only if you drive the generator from code.

7. Function signatures you may be calling directly.
   - `bgworker::pg::get_jobs` now returns loco_rs::Result instead of
     Result<_, sqlx::Error>. Code using `?` in a Loco context is unaffected;
     code matching on sqlx::Error needs updating.
   - views::tera_builtins::filters::number::{number_with_delimiter,
     number_to_human_size, number_to_percentage} take (value, Kwargs, &State).
     Templates are unaffected — only direct Rust calls need updating.
   - `build_with_post_process` now runs BEFORE templates are loaded. The
     closure registers into an empty engine and can no longer inspect loaded
     templates, and anything a template calls must be registered by it.

8. `auth.jwt.location` in config is parsed strictly now.
   It accepts a map, or a list of maps, and nothing else. The documented
   shapes are unchanged, but config that used to slip through an untagged
   fallback is now rejected with an error naming the problem.

9. OPTIONAL, no deadline: config templating delimiters.
   `{{ get_env(...) }}` still renders, with a deprecation warning. The
   YAML-safe form is:
       port: <%= get_env(name="PORT", default="5150") %>
   `{` is a YAML flow-mapping indicator, so the old form was never valid YAML
   at rest and format-on-save would rewrite it into `{ { ... } }` and break
   startup. This is a find-and-replace across config/*.yaml.

Do NOT make production config secrets mandatory. That change applies only to
NEWLY generated apps; an existing app's config files are its own.
````

### Єдина обов'язкова зміна: `fluent-templates`

Згенеровані застосунки використовують `fluent-templates` для i18n-функції
`t()`, а він закріплював Tera 1. Підніміть його у вашому `Cargo.toml`:

```toml
fluent-templates = { version = "0.15", features = ["tera"] }
```

Для більшості застосунків це **єдина** потрібна зміна. Ваш ініціалізатор
рушія перегляду — включно з
`tera.register_function("t", FluentLoader::new(arc.clone()))` — продовжує
працювати як є.

### Якщо ви реєструєте власні фільтри чи функції

Tera 2 змінив сигнатури. Фільтри тепер отримують `(Arg, Kwargs, &State)` над
власним типом `Value` Tera, замість `(&Value, &HashMap<String, Value>)` над
`serde_json::Value`:

```rust no-syntax-check="before/after signature comparison; the bodies are elided `{ ... }`"
// Tera 1
fn my_filter(value: &serde_json::Value, args: &HashMap<String, serde_json::Value>)
    -> tera::Result<serde_json::Value> { ... }

// Tera 2
fn my_filter(value: &tera::Value, kwargs: tera::Kwargs, _: &tera::State)
    -> tera::TeraResult<tera::Value> { ... }
```

Читайте іменовані аргументи через `kwargs.get::<T>("name")?` (опціональні)
або `kwargs.must_get::<T>("name")?` (обов'язкові).

Зверніть увагу, що `&State` не можна сконструювати поза рушієм, тож фільтри
більше не можна юніт-тестувати прямим викликом. Тримайте логіку форматування
у звичайній функції, а фільтр зробіть тонкою обгорткою над нею — та функція
залишиться тестованою.

### Якщо ваші шаблони перегляду використовують макроси чи доступ до масиву через крапку

Tera 2 прибрав `{% macro %}` / `{% import %}` на користь компонентів,
вимагає `v[0]` замість `v.0` для доступу до масиву та **видає помилку на
невизначених змінних**, замість того щоб рендерити їх порожніми. Шаблони, що
покладаються на будь-що з цього, потребують редагування. Згенеровані
застосунки Loco не використовують цих конструкцій.

### Файли конфігурації: `<%= ... %>`

Шаблонізація конфігурації переїхала з `{{ ... }}` Tera на `<%= ... %>`:

```yaml
port: <%= get_env(name="PORT", default="5150") %>
```

`{` є індикатором потокового відображення YAML, тож стара форма ніколи не була
дійсним YAML у спокої — редактори та форматувальники (prettier,
yaml-language-server, format-on-save) реструктуризували б її у `{ { ... } }`
і ламали запуск. `<` не є індикатором, тож нова форма є звичайним рядковим
скаляром і переживає форматування недоторканою.

Це **не** обов'язкова зміна: стара форма `{{ ... }}` досі рендериться, з
попередженням про застарілість. Конвертація — це пошук-і-заміна, коли вам
зручно.

## Оновлення з 0.16.x до 1.0

1.0 — це великий, навмисно ламаючий реліз — перший стабільний Loco. Його
головна зміна — перехід на **Sea-ORM 2.0**. Цей розділ зібрано за областями;
почніть з кроків Sea-ORM, які стосуються кожного застосунку, що використовує
базу даних. Звіряйте з [CHANGELOG 1.0.0](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md)
усе специфічне для API, які ви використовуєте безпосередньо.

### Інструментарій: Rust 1.94+

Loco 1.0 використовує Sea-ORM 2.0, чий MSRV — **Rust 1.94**. Оновіть ваш
інструментарій:

```sh
rustup update
```

### Sea-ORM 2.0

Loco оновився з Sea-ORM 1.1 до Sea-ORM 2.0. Для більшості застосунків
міграція механічна — підніміть закріплені версії та CLI — бо допоміжні
засоби `schema` Loco та згенеровані форми моделей/міграцій поглинають зміни
API за вас.

**1. Підніміть закріплені версії залежностей.** У `Cargo.toml` вашого застосунку:

```toml
# before
sea-orm = { version = "1.1", features = ["sqlx-sqlite", "sqlx-postgres", "runtime-tokio-rustls", "macros"] }
# after
sea-orm = { version = "2.0", features = ["sqlx-sqlite", "sqlx-postgres", "runtime-tokio-rustls", "macros"] }
```

І у вашому `migration/Cargo.toml`:

```toml
# before
sea-orm-migration = { version = "1.1.0", features = [...] }
# after
sea-orm-migration = { version = "2.0", features = [...] }
```

Якщо ви залежите від `sqlx` безпосередньо, підніміть його до `0.9`.

**2. Оновіть Sea-ORM CLI** (використовується `cargo loco db entities`) до 2.0:

```sh
cargo install sea-orm-cli --version '^2.0'
```

`cargo loco doctor` тепер позначатиме Sea-ORM чи Sea-ORM CLI старші за
`2.0.0-rc` (`src/doctor.rs:39,52`) — будь-який реліз-кандидат 2.0 задовольняє
перевірку, тож підніміться до стабільного 2.0 самостійно, а не чекайте, доки
doctor вам про це скаже.

**3. Перегенеруйте сутності (рекомендовано).** Запустіть `cargo loco db entities`,
щоб ваші `src/models/_entities/` були створені кодогенератором 2.0.

**4. Написані вручну запити / міграції.** Якщо ви писали власний сирий SQL чи
власні міграції, застосуйте ці зміни Sea-ORM 2.0 (ті самі, що зробив сам Loco):

- Виклики сирого `Statement` отримують суфікс `_raw`. `db.execute(stmt)` →
  `db.execute_raw(stmt)`; `db.query_one(stmt)` / `db.query_all(stmt)` →
  `query_one_raw` / `query_all_raw`. Інструкції SeaQuery (напр. з
  `Entity::find().into_query()`) тепер передаються **за посиланням** і не
  потребують ручного `.build(...)`: `db.query_all(&select)`.
- `sqlx` 0.9 вимагає, щоб SQL-рядки, побудовані під час виконання, були
  загорнуті в `AssertSqlSafe(...)`: `sqlx::query(AssertSqlSafe(format!(...)))`.
- Додайте `ExprTrait` у зону видимості для методів виразів: `use sea_orm::ExprTrait;`.
  Замініть `Alias::new("col")` на голий рядок `"col"`.
- Гілки для непідтримуваних бекендів повинні повертати
  `DbErr::BackendNotSupported { .. }`, а не панікувати; Sea-ORM 2.0 прибрав
  внутрішні паніки (новий варіант `DbErr` несе цей випадок). Якщо ви робите
  вичерпний `match` на `DbErr`, додайте цю гілку.
- `insert_many` більше не потребує `.on_empty_do_nothing()`, а
  `exec_with_returning_many` тепер називається `exec_with_returning`.

**5. Примітка щодо Postgres auto-increment.** Sea-ORM 2.0 випромінює
`GENERATED BY DEFAULT AS IDENTITY` замість `SERIAL` для нових колонок з
`auto_increment()`. Наявні таблиці не зачеплені; відрізняються лише ново
згенеровані міграції. Дивіться посібник з міграції Sea-ORM 2.0 щодо
аварійного люка `option-postgres-use-serial`, якщо вам потрібна стара
поведінка.

Для повних upstream-деталей дивіться
[посібник з міграції Sea-ORM 2.0](https://www.sea-ql.org/blog/2026-01-12-sea-orm-2.0/).

### Згенерований код тепер використовує 64-бітні первинні ключі

Ново **згенеровані** моделі та скафолди тепер використовують первинні та
зовнішні ключі `i64` (BIGINT), а типи полів `int`/`unsigned` генерують
64-бітні колонки. Це потрібно Sea-ORM 2.0 (його кодогенератор відображає
цілі числа SQLite на `i64`) і відповідає сучасній конвенції bigint за
замовчуванням.

Це стосується лише коду, який ви генеруєте *після* оновлення — ваші наявні
таблиці, міграції та сутності недоторкані. Якщо ви скафолдите нові ресурси і
хочете, щоб вони були пов'язані зі старішими таблицями з ключами `i32`,
зробіть типи ключів однаковими (або розширте старі міграцією, або вручну
відредагуйте нові поля `id`/зовнішніх ключів назад до `i32`).

### Кілька баз даних: `ExtraDbInitializer` → `MultiDbInitializer`

Ініціалізатор одного додаткового з'єднання (`initializers.extra_db`, який
додавав голий `Extension<DatabaseConnection>`) було вилучено. Використовуйте
`MultiDbInitializer` з відображенням `initializers.multi_db` з одним записом,
і витягуйте з'єднання через `Extension<MultiDb>`:

```rust
// before: Extension<DatabaseConnection>
// after:
let conn = multi_db.get("<name>")?;
```

Перенесіть усе, що ви налаштовували під `extra_db`, у відображення `multi_db`
з одним записом.

### `AppContext` тепер `#[non_exhaustive]` — конструюйте через builder

Доступ до полів (`ctx.db`, `ctx.config`, екстракція через `State`/`FromRef`)
не змінився, тож більшості застосунків зміна не потрібна. Але пряме
конструювання структурним літералом та вичерпні зіставлення зразків на
`AppContext` з-поза фреймворку більше не компілюються (це робить додавання
майбутніх полів контексту таким, що не ламає сумісність). Якщо ви будували
`AppContext` вручну — напр. у власному завантаженні чи тестовому
інструментарії — використовуйте builder:

```rust
let ctx = AppContext::builder(environment, db, config)   // builder(environment, config) without `with-db`
    .queue_provider(queue)
    .mailer(mailer)
    .storage(storage)
    .build();
```

### `loco_rs::Error` тепер `#[non_exhaustive]`

Енум `Error` фреймворку позначений `#[non_exhaustive]`, тож нові варіанти
можна додавати в майбутньому без зміни, що ламає сумісність. Якщо ви робите
вичерпний `match` на `loco_rs::Error` (чи `loco_rs::prelude::Error`), додайте
гілку з wildcard:

```rust
match err {
    Error::NotFound => { /* ... */ }
    // ...handle the variants you care about...
    _ => { /* fallback */ }
}
```

Більшість застосунків використовують `Result<T>` / `?` і ніколи не
зіставляються з `Error` напряму, тож зміна не потрібна.

### Точніші HTTP-коди станів для помилок

`IntoResponse for Error` раніше згортав більшість варіантів у `500`. Тепер
`Model(EntityNotFound)` → `404`, `Model(EntityAlreadyExists)` → `409`, а
валідація моделі / відхилення тіла форми → `4xx` (відповідаючи JSON-відхиленням);
справді внутрішні помилки досі повертають `500`. Це зміна лише поведінки —
жодне API не змінилося — але якщо ваші тести стверджували старі `500`,
оновіть їх до виправлених кодів.

### Пріоритети фонових задач (бекенд Redis ламає сумісність)

Фонові задачі тепер підтримують **пріоритет** (більші числа виконуються
першими). Ви можете ставити в чергу з явним пріоритетом:

```rust
DownloadWorker::perform_later_with_priority(&ctx, args, Some(42)).await?;
```

- **Postgres / SQLite: дій не потрібно.** Колонка `priority` додається до
  таблиці черги автоматично під час запуску; наявні задачі отримують
  стандартний пріоритет `0`.
- **Redis: ламає сумісність.** Щоб упорядковувати за пріоритетом, бекенд Redis
  тепер зберігає чергу як **Sorted Set (ZSET)** замість List. Задачі, що вже
  сидять у старих ключах черги на основі List, не будуть підхоплені після
  оновлення. **Розрядіть ваші черги Redis перед розгортанням 1.0** (дайте
  воркерам завершити задачі в роботі на старій версії, або очистіть чергу,
  якщо можете повторно поставити задачі в чергу). Ново поставлені в чергу
  задачі автоматично використовують формат ZSET.

Задачі поштовика ставляться в чергу з пріоритетом `100` за замовчуванням;
перевизначте на рівні поштовика через `MailerOpts { priority, .. }`.

### `perform_later` повертає id задачі

`Worker::perform_later` тепер повертає id поставленої в чергу задачі
(`Result<String>` замість `Result<()>`), а `Queue::enqueue` повертає
`Result<Option<String>>`. Наявні місця виклику продовжують працювати —
`perform_later(..).await?;` просто ігнорує повернений id. Захопіть його, коли
хочете відстежувати статус:

```rust
let job_id = DownloadWorker::perform_later(&ctx, args).await?;
```

### Фонова черга тепер є адаптером `QueueProvider`

`bgworker::Queue` тепер є newtype над `Arc<dyn QueueProvider>`, тож бекенди
стати підключуваними. Усі методи черги зберігають ті самі сигнатури та
поведінку. Лише дві зміни на рівні вихідного коду стосуються викликачів:

- Конструюйте no-op чергу через `Queue::empty()` замість `Queue::None`.
- Код, що зіставлявся з варіантами енуму (напр. `Queue::Postgres(pool, ..)`,
  щоб дістатися до сирого пулу), більше не компілюється — використовуйте
  натомість методи провайдера.

### `PageResponse` несе `meta: PagerMeta`

Результати пагінації перенесли плоскі поля `total_pages` / `total_items` у
`meta: PagerMeta` (яке також несе `page` та `page_size`):

```rust
// before
let total = page.total_pages;
// after
let total = page.meta.total_pages;   // also: page.meta.page, page.meta.page_size, page.meta.total_items
```

### Сховище: `MirrorStrategy` / `BackupStrategy` → `ReplicatedStrategy`

Дві стратегії були тим самим рушієм реплікації primary-plus-secondaries і
тепер є одним `storage::strategies::replicated::ReplicatedStrategy` з єдиним
енумом `FailurePolicy`:

```rust
// MirrorStrategy::new(p, s, MirrorAll)  ->
ReplicatedStrategy::mirror(p, s, FailurePolicy::FailIfAny);
// BackupStrategy::new(p, s, BackupAll)  ->
ReplicatedStrategy::backup(p, s, FailurePolicy::FailIfAny);
```

Відображення старого `FailureMode`: `AllowMirrorFailure` / `AllowBackupFailure` → `AllowAll`,
`AtLeastOneFailure` → `AllowSingleFailure`, `CountFailure(n)` → `FailAtFailures(n)`.
Записи вторинних сховищ, що раніше були backup, тепер виконуються конкурентно
(раніше послідовно); зібрані помилки та рішення про невдачу не змінилися.

### Сховище: локальний драйвер більше не вкорінюється в `/` (безпека)

`storage::drivers::local::new()` раніше вкорінював сховище в `/`, тож ключ,
похідний від користувацького вводу, міг утекти на весь диск (ключ `etc/passwd`
читав `/etc/passwd`). Тепер він вкорінюється в поточну робочу директорію. Якщо
ви покладалися на ключі з абсолютними шляхами, явно увімкніть це назад:

```rust
local::new_with_prefix("/your/root")
```

### Конфігурація: `{env}.local.yaml` тепер глибоко зливається поверх `{env}.yaml`

Раніше перемагав перший наявний файл, а другий ігнорувався, тож `.local.yaml`
мав повторювати всю конфігурацію. Обидва файли тепер нашаровуються з
пріоритетом локального: відображення зливаються рекурсивно; скаляри та
послідовності в локальному замінюють базове значення (послідовності **не**
конкатенуються). Якщо ви тримали `.local.yaml` з повною конфігурацією,
скоротіть його лише до ключів, які ви перевизначаєте — базові ключі тепер
зберігаються, якщо їх явно не перевизначено.

### Fallback middleware за замовчуванням повертає `404`

Коли вбудований fallback увімкнений без явного `code`, він тепер повертає
`404 Not Found` (відповідаючи своїй документації та вбудованій сторінці
not-found) замість `200 OK`. Якщо ви покладалися на увімкнений fallback, що
повертає `200`, встановіть `code: 200` явно. Файловий fallback (`ServeFile`)
не зачеплений.

### `remote_ip` перебудовано на `axum-client-ip`; `trusted_proxies` вилучено (безпека)

**Це мовчазна, безпеково релевантна зміна.** Ключ `trusted_proxies:` зі старої
конфігурації тепер є невідомим полем і **ігнорується без помилки**, тож
перегляньте свою конфігурацію `remote_ip` перед оновленням — вона не зазнає
невдачі при завантаженні.

Раніше middleware проходив `X-Forwarded-For` справа наліво, пропускаючи
будь-яку адресу в списку CIDR `trusted_proxies` (або вбудованому списку
RFC-1918 + loopback). Тепер він довіряє рівно **одному** налаштованому
джерелу (`source: ClientIpSource`, стандарт `RightmostXForwardedFor`) і **не**
виконує жодної CIDR-фільтрації.

- **Розгортання з одним зворотним проксі:** не зачеплені.
- **Топології з кількома стрибками (CDN → LB → ingress):** налаштуйте свій
  найвнутрішніший стрибок, щоб він встановлював IP клієнта (напр. nginx
  `set_real_ip_from` / `real_ip_recursive`), або вкажіть `source` на заголовок
  провайдера (`CfConnectingIp`, `CloudFrontViewerAddress`, `XRealIp`,
  `ConnectInfo`, …).

Екстрактор `RemoteIP` та його вивід `Display` не змінилися.

### JWT: `algorithm()` обмежений родиною HMAC

`JWT::algorithm()` тепер приймає `loco_rs::auth::jwt::JWTAlgorithm`
(`HS256` / `HS384` / `HS512`) замість `jsonwebtoken::Algorithm`. Асиметричні
алгоритми — які ніколи не могли працювати зі спільним base64-секретом Loco і
мовчки виробляли зламані токени — більше не репрезентовані. Якщо ви
передавали `jsonwebtoken::Algorithm`, перейдіть на відповідний варіант
`JWTAlgorithm`.

### Рушій перегляду: використовуйте `TeraView::build_with_post_process`

У `after_routes` замініть `TeraView::build()?.post_process(...)` на
комбінований конструктор:

```rust no-syntax-check="before/after expression fragments, not a whole item"
// before
engines::TeraView::build()?.post_process(move |tera| {
    tera.register_function("t", FluentLoader::new(arc.clone()));
    Ok(())
})?
// after
engines::TeraView::build_with_post_process(move |tera| {
    tera.register_function("t", FluentLoader::new(arc.clone()));
    Ok(())
})?
```

### Поштовик: `Template::new(dir)` тепер повертає `Result`

Шаблони електронної пошти рендеряться через повноцінний екземпляр Tera (тож
вони підтримують наслідування та спільні шаблони). Стандартне використання
через `Mailer::mail_template` не змінилося; якщо ви викликали
`Template::new(dir)` напряму, додайте `?`:

```rust
let tpl = Template::new(dir)?;
```

### Задачі: `Vars::cli_arg` повертає `Result<&str>`

`Vars::cli_arg` тепер повертає `Result<&str>` (було `Result<&String>`).
Викликачі, що покладалися на `&String` (напр. `.clone()` у `String`),
повинні використовувати `.to_owned()`.

### Мажорні версії залежностей

1.0 піднімає кілька мажорних версій залежностей. Для більшості застосунків
вони транзитивні — вам потрібно діяти лише якщо ви використовуєте один із цих
крейтів **безпосередньо** через публічне API Loco: `thiserror` 1→2, `tower`
0.4→0.5, `heck`→0.5, `byte-unit` 4→5, `ipnetwork` 0.20→0.21, `strum`→0.27,
`redis` 0.31→1, `bb8-redis`→0.26, `opendal` 0.54→0.57. `serde_yaml`
(заархівований) було замінено на підтримуваний форк `serde_yaml_ng`.

### Зміни feature-прапорів (1.0)

- `auth_jwt` → `auth`.
- `bg_redis` → `worker_redis`; `bg_pg`/`bg_sqlt` → `worker`. `default` тепер
  включає `worker` (черги Postgres+SQLite); додайте `worker_redis` для черги
  Redis.
- `integration_test` вилучено (був мертвим).
- `loco new` тепер пропонує бекенди черг Redis/Postgres/SQLite та (на боці
  сервера) вбудовані ресурси.

## Оновлення з 0.15.x до 0.16.x

### Використовуйте `AppContext` замість `Config` в `init_logger` у трейті `Hooks`

PR: [#1418](https://github.com/loco-rs/loco/pull/1418)

Якщо ви постачаєте реалізацію `init_logger` у вашому `impl` трейту `Hooks`,
щоб налаштувати власне логування, вам потрібно зробити таку зміну:

```diff
- fn init_logger(config: &config::Config, env: &Environment) -> Result<bool> {
+ fn init_logger(ctx: &AppContext) -> Result<bool> {
```

Будь-який код у вашій реалізації `init_logger`, що використовує `config`,
може отримати доступ до нього через `ctx.config`. Крім того, ви також зможете
отримати доступ до всього іншого в `AppContext`, напр. до нового
`shared_store`. Параметр `env` також вилучено, оскільки він доступний з
`AppContext` як `ctx.environment`.

### Перехід на вбудовану валідацію email з validators

PR: [#1359](https://github.com/loco-rs/loco/pull/1359)

Перейдіть з використання власного email-валідатора Loco на вбудований
email-валідатор з `validator`.

```diff
- #[validate(custom (function = "validation::is_valid_email"))]
+ #[validate(email(message = "invalid email"))]
  pub email: String,
```

### Система задач

PR: [#1384](https://github.com/loco-rs/loco/pull/1384)
PR: [#1396](https://github.com/loco-rs/loco/pull/1396)

До системи фонових задач було внесено дві великі зміни:

1. Провайдер Redis більше не сумісний з Sidekiq і використовує власну реалізацію
2. Усі провайдери (Redis, PostgreSQL, SQLite) тепер підтримують фільтрацію задач за тегами

#### Що змінилося

##### Вилучення сумісності з Sidekiq

Система фонових задач Redis була повністю відрефакторена, замінивши
реалізацію, сумісну з Sidekiq, на нову власну реалізацію. Це дає більшу
гнучкість і покращену продуктивність, але означає:

- Задачі, поставлені в чергу зі старіших версій Loco (до 0.16), не будуть
  розпізнані чи оброблені
- Структури даних Redis повністю змінилися
- Автоматичного шляху міграції для наявних задач у черзі немає

##### Додавання фільтрації задач

Нова система фільтрації задач за тегами була додана до всіх провайдерів
фонових воркерів:

- Воркери тепер можуть вказувати, які теги їх цікавлять для обробки
- Задачі можна позначати тегами під час постановки в чергу
- Воркери без тегів обробляють лише задачі без тегів, тоді як воркери з
  тегами обробляють задачі зі збіжними тегами
- Те саме API використовується в усіх провайдерах

#### Як оновитися

Щоб оновитися до нової системи задач:

1. **Обробіть наявні задачі**:

   - Переконайтеся, що всі задачі у вашій черзі оброблені/завершені перед оновленням

2. **Очистіть старі дані**:

   - Для Redis: очистіть базу даних Redis, що використовується для задач (команда `FLUSHDB`)
   - Для PostgreSQL: видаліть таблиці черги задач
   - Для SQLite: видаліть таблиці черги задач

3. **Оновіть Loco**:
   - Оновіться до Loco 0.16+
   - Loco автоматично створить нові таблиці задач з правильною схемою під час першого запуску

### Узагальнений кеш

PR: [#1385](https://github.com/loco-rs/loco/pull/1385)

API кешу було відрефакторено для підтримки збереження та отримання будь-якого
серіалізованого типу, а не лише рядків. Це зміна, що ламає сумісність і
вимагає оновлень вашого коду:

#### Зміни, що ламають сумісність:

1. **Потрібні параметри типів**: усі методи кешу тепер вимагають явних параметрів типів
2. **Сигнатури методів**: деякі сигнатури методів змінилися для підтримки дженериків
3. **Серіалізація об'єктів**: будь-який тип, який ви зберігаєте, має реалізовувати `Serialize` та `Deserialize` з serde

#### Посібник з міграції:

**Було:**

```rust
// Get a string value from cache
let value = cache.get("key").await?;

// Insert or get with callback
let value = app_ctx.cache.get_or_insert("key", async {
    Ok("value".to_string())
}).await.unwrap();

// Insert or get with expiry
let value = app_ctx.cache.get_or_insert_with_expiry("key", Duration::from_secs(300), async {
    Ok("value".to_string())
}).await.unwrap();
```

**Стало:**

```rust
// Get a string value from cache - specify the type
let value = cache.get::<String>("key").await?;

// Direct insert with any serializable type
cache.insert("key", &"value".to_string()).await?;

// Insert or get with callback - specify return type
let value = app_ctx.cache.get_or_insert::<String, _>("key", async {
    Ok("value".to_string())
}).await.unwrap();

// Store complex types
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

let user = app_ctx.cache.get_or_insert_with_expiry::<User, _>(
    "user:1",
    Duration::from_secs(300),
    async {
        Ok(User { name: "Alice".to_string(), age: 30 })
    }
).await.unwrap();
```

#### Реалізація для власних типів:

Щоб ваші власні типи працювали з кешем, переконайтеся, що вони реалізовують
`Serialize` та `Deserialize`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyType {
    // fields...
}
```

### Обробка помилок автентифікації

Обробка помилок автентифікації була покращена, щоб краще розрізняти
справжні збої авторизації та системні помилки:

1. **Системні помилки тепер повертають 500**: помилки бази даних під час
   автентифікації тепер повертають Internal Server Error (500) замість
   Unauthorized (401)
2. **Покращене логування помилок**: помилки автентифікації тепер логуються з
   детальними повідомленнями через `tracing::error`
3. **Зміни повідомлень**: загальні повідомлення про помилки оновлено з
   "other error: '{e}'" на "could not authorize"

#### Посібник з міграції:

Якщо у вас є код, що покладається на повернення кодів стану 401 помилками
бази даних під час автентифікації, вам потрібно оновити свою обробку помилок.
Будь-який код, що очікує 401 для проблем з'єднання з базою даних, тепер
повинен також обробляти відповіді 500.

Клієнтські застосунки мають бути готові обробляти обидва коди стану, 401 та
500, під час збоїв автентифікації: 401 вказує на проблеми авторизації, а
500 — на системні помилки.

### Рендеринг на боці сервера
Ми внесли деякі зміни до Tera-шаблону. Перейдіть до
`src/initializers/view_engine.rs` і замініть функцію `after_routes` на:
```rust
async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let tera_engine = if std::path::Path::new(I18N_DIR).exists() {
            let arc = std::sync::Arc::new(
                ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                    .shared_resources(Some(&[I18N_SHARED.into()]))
                    .customize(|bundle| bundle.set_use_isolating(false))
                    .build()
                    .map_err(|e| Error::string(&e.to_string()))?,
            );
            info!("locales loaded");

            engines::TeraView::build()?.post_process(move |tera| {
                tera.register_function("t", FluentLoader::new(arc.clone()));
                Ok(())
            })?
        } else {
            engines::TeraView::build()?
        };

        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
```

## Оновлення з 0.14.x до 0.15.x

### Оновлення крейту validator

PR: [#1199](https://github.com/loco-rs/loco/pull/1199)

Оновіть версію крейту `validator` у вашому `Cargo.toml`:

З

```
validator = { version = "0.19" }
```

На

```
validator = { version = "0.20" }
```

### Claims користувача

PR: [#1159](https://github.com/loco-rs/loco/pull/1159)

- Пласке (де)серіалізування власних Claims користувача:
  поле `claims` у `UserClaims` змінилося з `Option<Value>` на `Map<String, Value>`.

- Обов'язкове значення мапи у функції `generate_token`:
  під час виклику `generate_token` аргумент `Map<String, Value>` тепер є
  обов'язковим. Якщо ви не використовуєте власні claims, передайте порожню
  мапу (`serde_json::Map::new()`).

- Оновлена сигнатура generate_token:
  функція `generate_token` тепер приймає `expiration` як значення, а не як
  посилання.

### Відповідь пагінації

PR: [#1197](https://github.com/loco-rs/loco/pull/1197)

Відповідь пагінації тепер включає поле `total_items`, що надає загальну
кількість доступних елементів.

```JSON
{"results":[],"pagination":{"page":0,"page_size":0,"total_pages":0,"total_items":0}}
```

### Явний id у міграціях

PR: [#1268](https://github.com/loco-rs/loco/pull/1268)

Міграції, що використовують `create_table`, тепер вимагають
`("id", ColType::PkAuto)`; нові міграції будуть мати це поле, додане
автоматично.

```diff
  async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(m, "movies",
            &[
+           ("id", ColType::PkAuto),
            ("title", ColType::StringNull),
            ],
            &[
            ("user", ""),
            ]
        ).await
    }
```

## Оновлення з 0.13.x до 0.14.x

### Оновлення з Axum 0.7 до 0.8

PR: [#1130](https://github.com/loco-rs/loco/pull/1130)
Оновлення до Axum 0.8 вводить зміну, що ламає сумісність. Для більше деталей
зверніться до [анонсу](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0).

#### Кроки для оновлення

- У вашому `Cargo.toml` оновіть версію Axum з `0.7.5` до `0.8.1`.
- Замініть `use axum::async_trait;` на `use async_trait::async_trait;`. Для
  більше інформації дивіться [тут](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#async_trait-removal).
- Синтаксис параметрів URL змінився. Зверніться до
  [цього розділу](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0#path-parameter-syntax-changes)
  щодо оновленого синтаксису. Новий формат параметрів шляху:
  Синтаксис параметрів шляху змінився з `/:single` та `/*many` на `/{single}` та `/{*many}`.

### Розширення хука функції `boot`

PR: [#1143](https://github.com/loco-rs/loco/pull/1143)

Хук-функція `boot` тепер приймає додатковий параметр Config. Сигнатура
функції змінилася з:

З

```rust
async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
     create_app::<Self, Migrator>(mode, environment).await
}
```

На:

```rust
async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult> {
     create_app::<Self, Migrator>(mode, environment, config).await
}
```

Переконайтеся, що імпортували тип `Config`, за потреби.

### Оновлення крейту validator

PR: [#993](https://github.com/loco-rs/loco/pull/993)

Оновіть версію крейту `validator` у вашому `Cargo.toml`:

З

```
validator = { version = "0.18" }
```

На

```
validator = { version = "0.19" }
```

### Розширення хуків truncate та seed

PR: [#1158](https://github.com/loco-rs/loco/pull/1158)

Функції `truncate` та `seed` тепер отримують `AppContext` замість
`DatabaseConnection` як аргумент.

З

```rust
async fn truncate(db: &DatabaseConnection) -> Result<()> {}
async fn seed(db: &DatabaseConnection, base: &Path) -> Result<()> {}
```

На

```rust
async fn truncate(ctx: &AppContext) -> Result<()> {}
async fn seed(_ctx: &AppContext, base: &Path) -> Result<()> {}
```

Вплив на тестування:

Тестовий код, що залучає функцію seed, також має бути оновлений
відповідно.

з:

```rust no-syntax-check="`...` elides the rest of the closure body"
async fn load_page() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx.db).await.unwrap();
        ...
    })
    .await;
}
```

на:

```rust no-syntax-check="`...` elides the rest of the closure body"
async fn load_page() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        ...
    })
    .await;
}
```
