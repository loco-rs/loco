---
title: Генератори та типи полів
description: Усі компоненти `cargo loco generate <kind>`, їхній точний CLI-синтаксис і файли виводу, а також повна мінімова типів полів, яку використовують генератори model/migration/scaffold.
sidebar:
  order: 3
---

`cargo loco generate <kind>` (аліас `cargo loco g <kind>`) створює каркас коду застосунку з шаблонів, вшитих у крейт `loco-gen`. Сама підкоманда `generate` компілюється лише під `#[cfg(debug_assertions)]` (`src/cli.rs:140`) — вона доступна у звичайних (dev/debug) збірках, але виключається з `--release`-бінарників. Види, що торкаються бази даних (`model`, `migration`, `scaffold`), додатково обмежені Cargo-ознакою `with-db` (увімкнена за замовчуванням) — див. [ознаки (feature flags)](/uk/docs/reference/feature-flags/).

Ця сторінка — вичерпний словник видів генераторів і мінімови типів полів (`name:type`), яку вони всі спільно використовують. Вона транскрибує `loco-gen/src/lib.rs` (перелік `Component`), `loco-gen/src/column.rs` (модель типу/колонки поля), `loco-gen/src/infer.rs` (конвенції іменування/словозміни) та `src/cli.rs` (поверхня CLI), повторно перевірені проти `HEAD`.

## Види генераторів

Перелік `Component`: `loco-gen/src/lib.rs:237`. Перелік підкоманд CLI `ComponentArg`: `src/cli.rs:175` (також обмежений `#[cfg(debug_assertions)]`).

| Вид | CLI-синтаксис | Обмеження ознакою | Примітки |
|---|---|---|---|
| **model** | `cargo loco generate model <name> [field:type ...] [--without-tz]` | `with-db` | Створює сутність Sea-ORM + файл моделі + міграцію + тести. `lib.rs:239`, `cli.rs:197` |
| **migration** | `cargo loco generate migration <name> [field:type ...] [--without-tz]` | `with-db` | Окремий файл міграції; без моделі/сутності. Виведення операції з назви (create/add/remove/join) — див. [Виведення з назви міграції](#виведення-з-назви-міграції). `lib.rs:250`, `cli.rs:250` |
| **scaffold** | `cargo loco generate scaffold <name> [field:type ...] [--without-tz] [--no-auth]` | `with-db` | Повний CRUD: сутність, міграція, DTO, контролер, маршрути та тест моделі — плюс типізовані React-хуки/сторінки, коли застосунок має `frontend/`. Адаптивний, без прапорця виду. `lib.rs:102`, `cli.rs:268` |
| **controller** | `cargo loco generate controller <name> [action ...] [--auth]` | немає | Контролер (JSON API) + маршрути + request-тест — без моделі/міграції. `lib.rs:117`, `cli.rs:299` |
| **task** | `cargo loco generate task <name>` | немає | Заготовка одноразової/CLI задачі, реєструється в `src/tasks/mod.rs`. `lib.rs:284` |
| **scheduler** | `cargo loco generate scheduler` | немає | Створює `config/scheduler.yaml`. `lib.rs:288` |
| **worker** | `cargo loco generate worker <name>` | немає | Заготовка фонового робітника в `src/workers/`, реєструється в `src/workers/mod.rs`. `lib.rs:289` |
| **mailer** | `cargo loco generate mailer <name>` | немає | Структура mailer у `src/mailers/<name>.rs` + вбудовані шаблони `welcome/{subject,html,text}.t`. `lib.rs:293` |
| **data** | `cargo loco generate data <name>` | немає | Структура завантажувача даних + статичний файл `data/<name>/data.json`. `lib.rs:297` |
| **deployment** | `cargo loco generate deployment <docker\|nginx\|lambda>` | немає | `kind` — **позиційне** значення, а не прапорець `--kind` (див. [Deployment](#deployment)). |
| **override** | `cargo loco generate override [template_path] [--info]` | немає | Копіює вбудовані шаблони до `.loco-templates/` застосунку, щоб ви могли їх кастомізувати. `cli.rs:373` |

### Model, migration, scaffold

Усі три приймають `name` і список пар `field:type` (див. [мінімову типів полів](#мінімова-типів-полів) нижче) та прапорець `--without-tz`, щоб пропустити колонки часових міток `created_at`/`updated_at`. Назви полів `created_at`, `updated_at`, `create_at`, `update_at` мовчки пропускаються, якщо ви передасте їх явно (`IGNORE_FIELDS`, `loco-gen/src/model.rs:16`) — вони генеруються автоматично.

```bash
# порожня модель
cargo loco generate model posts

# модель з полями
cargo loco generate model posts title:string! content:text

# модель із посиланням belongs-to (додає FK-колонку `director_id` на `movies`)
cargo loco generate model movies long_title:string director:references award:references:prize_id

# міграція, що додає колонки до наявної таблиці
cargo loco generate migration AddNameAndAgeToUsers name:string age:int

# scaffold (модель + DTO + контролер; додає React-хуки/сторінки, якщо застосунок має frontend/)
cargo loco generate scaffold posts title:string! user:references
```

Після генерації `migration` застосуйте її та перегенеруйте сутності: `cargo loco db migrate && cargo loco db entities`.

### Вид scaffold / controller

**Прапорця виду немає.** Генератори 1.0 є адаптивними: `controller` завжди генерує контролер JSON API, а `scaffold` генерує JSON API плюс — коли застосунок має `frontend/` (клієнтський React SPA) — типізовані React Query-хуки та сторінки для ресурсу. Headless-застосунки отримують лише бекенд. Scaffold визначає це за `frontend/src/routes.tsx` (`src/cli.rs`, `Component::Scaffold { frontend }`). Про те, що генерує frontend-частина і як типи TypeScript залишаються синхронними з вашими Rust DTO, див. [Побудувати типізований React SPA](/uk/docs/how-to/build-a-spa/).

### Автентифікація на згенерованих маршрутах

**Scaffold за замовчуванням автентифікований**: усі п'ять CRUD-обробників приймають екстрактор `auth::JWT`, тож анонімний запит отримує `401 Unauthorized`. Це навмисно — під scaffold-ресурсом лежить справжня таблиця, і випадково відкрити її — дорожча помилка. Передайте **`--no-auth`**, щоб згенерувати той самий контролер з публічними маршрутами:

```bash
# автентифіковано (за замовчуванням) — вимагає `Authorization: Bearer <token>`
cargo loco generate scaffold posts title:string!

# публічно
cargo loco generate scaffold posts title:string! --no-auth
```

Автентифікований scaffold виводить однорядкове нагадування, що його маршрути вимагають JWT, і вказує на `--no-auth`; сам `--no-auth` виводить лише звичайний рядок «controller was added» (примітка лежить всередині `{% if auth %}` у `scaffold/api/controller.t:3`). У будь-якому разі React-frontend-частина не змінюється: SPA надсилає свій bearer-токен, коли має його, а публічний API його ігнорує.

Згенерований **controller за замовчуванням публічний** — за ним ще немає моделі, тож захищати нічого, доки ви не напишете тіла обробників. **`--auth`** — дзеркальна опція, яка додає той самий екстрактор `auth::JWT` до кожного обробника (і генерує request-тест, що перевіряє відхилення анонімних викликів):

```bash
cargo loco generate controller posts --auth
```

Щоб додати чи прибрати auth після генерації, додайте або видаліть аргумент `_auth: auth::JWT,` на потрібних обробниках — більше нічого в контролері від нього не залежить. Про те, як видається токен і звідки його читають, див. [JWT-автентифікація](/uk/docs/how-to/jwt-auth/) та [Розташування JWT](/uk/docs/how-to/jwt-locations/).

Домонові прапорці `--api` / `--html` / `--htmx` (разом з `-k/--kind` і переліком `ScaffoldKind`) було вилучено з адаптивною перебудовою. Для зворотної сумісності генератори досі **приймають** `--api` (нічого не робить — це headless-стандарт) та `--html`/`--htmx` (які повертають помилку з вказівкою на React SPA frontend, що замінив server-rendered views), тож наявні туторіали й скрипти не падають із clap-помилкою `unexpected argument` (`warn_legacy_scaffold_kind`, `src/cli.rs`).

### Deployment

`DeploymentKind` у `loco-gen` несе дані генератора (`loco-gen/src/lib.rs:57-75`):

```rust
pub enum DeploymentKind {
    Docker { copy_paths: Vec<PathBuf>, is_client_side_rendering: bool },
    Nginx { host: String, port: i32 },
    Lambda { db: bool, include_paths: Vec<PathBuf> },
}
```

але **видимий з CLI** перелік (`src/cli.rs:568-573`) — це звичайний `clap::ValueEnum { Docker, Nginx, Lambda }`, що приймається як позиційний аргумент — кожне поле корисного навантаження (`copy_paths`, `is_client_side_rendering`, `host`, `port` та `db` + `include_paths` для Lambda) виводиться з власного `config/*.yaml` застосунку та файлової системи на момент генерації, а не передається з командного рядка:

```bash
cargo loco generate deployment docker   # створює Dockerfile, .dockerignore
cargo loco generate deployment nginx    # створює nginx/default.conf
cargo loco generate deployment lambda   # створює src/bin/lambda.rs, додає lambda_http
```

### Override

Копіює вбудований шаблон `.t` (або цілий каталог) до локальної директорії `.loco-templates/` (`DEFAULT_LOCAL_TEMPLATE`, `loco-gen/src/template.rs:8`), тож наступні запуски генерації використовують вашу копію замість вбудованої. Видаліть локальну копію, щоб повернутися до вбудованого шаблону.

```bash
# перелічити всі шаблони, які можна перекрити
cargo loco generate override

# перекрити один файл
cargo loco generate override scaffold/api/controller.t

# перекрити всі шаблони в каталозі
cargo loco generate override scaffold/frontend

# переглянути, що показав би --info для каталогу, без копіювання
cargo loco generate override scaffold/api --info

# перекрити все
cargo loco generate override .
```

## Мінімова типів полів

Кожен аргумент `field:type` для `model`/`migration`/`scaffold` розв'язується в `loco-gen/src/column.rs` — функція `parse_column` і перелік `ScalarType`. Нижче повна транскрипція (повторно перевірена проти `HEAD`).

**Конвенція суфіксів:** без суфікса = nullable `Option<T>`; **`!`** = обов'язкове (non-null); **`^`** = унікальне (неявно non-null). Не кожен базовий тип має всі три варіанти — `bool`, `tstz` і `json` не мають форми `^` (унікальної).

Написання обох (`string!^`, `string^!`) приймається і означає те саме, що й `^` окремо, бо `^` уже неявно означає non-null. На параметризованому типі будь-який прапорець може також стояти на базовій назві перед параметрами — `decimal_len!:8:24` і `decimal_len:8:24!` — та сама колонка.

**Зміна 1.0:** `int` тепер відображається на **`i64` / `BIGINT`** (`big_integer`), відповідно до 64-бітних первинних ключів фреймворку. До 1.0 `int` був `i32`. `unsigned` — аліас `big_unsigned` (також i64). Використовуйте `small_int`/`small_unsigned`, якщо вам конкретно потрібні 16-бітні колонки.

| `type` (варіанти суфіксів) | Тип Rust | Варіант `ColType` | Арність |
|---|---|---|---|
| `uuid` / `uuid!` / `uuid^` | `Option<Uuid>` / `Uuid` / `Uuid` | `UuidNull` / `Uuid` / `UuidUniq` | — |
| `string` / `string!` / `string^` | `Option<String>` / `String` / `String` | `StringNull` / `String` / `StringUniq` | — |
| `text` / `text!` / `text^` | `Option<String>` / `String` / `String` | `TextNull` / `Text` / `TextUniq` | — |
| `small_int` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallIntegerNull` / `SmallInteger` / `SmallIntegerUniq` | — |
| `small_unsigned` / `!` / `^` | `Option<i16>` / `i16` / `i16` | `SmallIntegerNull` / `SmallInteger` / `SmallIntegerUniq` — той самий `ColType`, що й у `small_int`, див. нижче | — |
| `int` / `!` / `^` **(⚠ i64, до 1.0 було i32)** | `Option<i64>` / `i64` / `i64` | `BigIntegerNull` / `BigInteger` / `BigIntegerUniq` | — |
| `big_int` / `!` / `^` (аліас `int`) | `Option<i64>` / `i64` / `i64` | `BigIntegerNull` / `BigInteger` / `BigIntegerUniq` | — |
| `unsigned` / `!` / `^` (аліас `big_unsigned`) | `Option<i64>` / `i64` / `i64` | `BigUnsignedNull` / `BigUnsigned` / `BigUnsignedUniq` | — |
| `big_unsigned` / `!` / `^` | `Option<i64>` / `i64` / `i64` | `BigUnsignedNull` / `BigUnsigned` / `BigUnsignedUniq` | — |
| `float` / `!` / `^` | `Option<f32>` / `f32` / `f32` | `FloatNull` / `Float` / `FloatUniq` | — |
| `double` / `!` / `^` | `Option<f64>` / `f64` / `f64` | `DoubleNull` / `Double` / `DoubleUniq` | — |
| `decimal` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `DecimalNull` / `Decimal` / `DecimalUniq` | — |
| `decimal_len` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `DecimalLenNull` / `DecimalLen` / `DecimalLenUniq` | **2** (precision, scale) |
| `bool` / `!` (без `^`) | `Option<bool>` / `bool` | `BooleanNull` / `Boolean` | — |
| `tstz` / `!` (без `^`) | `Option<DateTimeWithTimeZone>` / `DateTimeWithTimeZone` | `TimestampWithTimeZoneNull` / `TimestampWithTimeZone` | — |
| `date` / `!` / `^` | `Option<Date>` / `Date` / `Date` | `DateNull` / `Date` / `DateUniq` | — |
| `time` / `!` / `^` | `Option<Time>` / `Time` / `Time` | `TimeNull` / `Time` / `TimeUniq` | — |
| `date_time` / `!` / `^` | `Option<DateTime>` / `DateTime` / `DateTime` | `DateTimeNull` / `DateTime` / `DateTimeUniq` | — |
| `json` / `!` (без `^`) | `Option<serde_json::Value>` / `serde_json::Value` | `JsonNull` / `Json` | — |
| `jsonb` / `!` / `^` | `Option<serde_json::Value>` / `serde_json::Value` / `serde_json::Value` | `JsonBinaryNull` / `JsonBinary` / `JsonBinaryUniq` | — |
| `blob` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BlobNull` / `Blob` / `BlobUniq` | — |
| `money` / `!` / `^` | `Option<Decimal>` / `Decimal` / `Decimal` | `MoneyNull` / `Money` / `MoneyUniq` | — |
| `binary_len` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `BinaryLenNull` / `BinaryLen` / `BinaryLenUniq` | **1** (довжина) |
| `var_binary` / `!` / `^` | `Option<Vec<u8>>` / `Vec<u8>` / `Vec<u8>` | `VarBinaryNull` / `VarBinary` / `VarBinaryUniq` | **1** (довжина) |
| `array` / `!` / `^` | `Option<Vec<T>>` (див. нижче) | `array_null` / `array` / `array_uniq` (генератор створює `ColType::array(ArrayColType::…)` тощо) | **1** (тип елемента) |
| `enum` / `!` / `^` | `Option<T>` / `T` / `T`, де `T` — PascalCase-однина *назви колонки* (`status` → `Status`) | `StringNull` / `String` / `StringUniq` — зберігається як строкова колонка | **1** (значення через кому, наприклад `status:enum:draft,published`) |

`decimal`, `money` і `decimal_len` усі розв'язуються в той самий тип Rust (`rust_decimal::Decimal`); `ColType` розрізняє SQL-подання.

`small_unsigned` навмисно створює **знаковий** `ColType` `SmallInteger*`, а не `SmallUnsigned` з sea-orm (`loco-gen/src/column.rs:465`). Ні SQLite, ні Postgres не мають нативних беззнакових цілих, а `SmallUnsigned` проходить туди-назад як `i16` на SQLite, але `i32` на Postgres — тож згенеровані модель і DTO не скомпілювалися б проти Postgres. `i16` збігається з DTO на обох бекендах.

### Enums

`enum` приймає значення одним параметром через кому: `status:enum:draft,published,archived`. Сама колонка в базі даних — звичайний рядок. Scaffold генерує справжній Rust-перелік поруч із DTO — названий за колонкою (`status` → `Status`), по одному варіанту на значення, `#[serde(rename_all = "snake_case")]`, з impl `From<String>` та ts-rs-експортом для frontend — а згенерована scaffold-форма рендерить поле як `<select>` з цими значеннями.

### Arrays

`array`/`array!`/`array^` приймає один параметр — тип елемента — записаний другим сегментом через двокрапку: `tags:array:string`, `scores:array!:int`. Допустимі типи елементів (за `array_inner_from_name` у `loco-gen/src/column.rs`): `string`, `int`, `big_int`, `float`, `double`, `bool`, що генерує `Option<Vec<T>>`, де `T`:

| елемент | Rust `T` |
|---|---|
| `string` | `String` |
| `int` | `i64` |
| `big_int` | `i64` |
| `float` | `f32` |
| `double` | `f64` |
| `bool` | `bool` |

**Примітка:** типи елементів масивів у 1.0 узгоджені зі скалярами — `array:int` генерує 64-бітний масив `BigInt` (тип елемента `i64`), відповідно до зміни скалярного `int` → `i64`, за `array_col_type_name` у `loco-gen/src/column.rs` (`ScalarType::Int | ScalarType::BigInt => "BigInt"`). Юніт-тест у `column.rs` фіксує це, перевіряючи `array:big_int!` → `array(ArrayColType::BigInt)`.

### References (зовнішні ключі belongs-to)

Поле типу `references` (немає в таблиці вище — обробляється окремо в `loco-gen/src/infer.rs:29-54`) генерує FK-колонку belongs-to замість звичайної колонки:

| Синтаксис | Значення |
|---|---|
| `name:references` | Обов'язковий FK до таблиці `names`, колонка `name_id` |
| `name:references:custom_id` | Обов'язковий FK, явна назва FK-колонки `custom_id` |
| `name:references?` | Nullable FK до таблиці `names` |
| `name:references?:custom_id` | Nullable FK, явна назва FK-колонки |

Приклад: `director:references award:references:prize_id` на моделі `movies` додає обов'язковий FK `director_id` до `directors` і обов'язковий FK `prize_id` до `awards`.

## Виведення з назви міграції

Для `cargo loco generate migration <Name> ...` функція `guess_migration_type` (`loco-gen/src/infer.rs:56`) зіставляє з шаблоном назву міграції, приведену до **snake_case**, щоб вирішити, що створювати:

| Шаблон назви | Виведена операція |
|---|---|
| `Create<Table>` | `CreateTable` |
| `Add<ref>RefTo<Table>` | `AddReference` |
| `Add<Columns>To<Table>` | `AddColumns` |
| `Remove<Columns>From<Table>` | `RemoveColumns` |
| `Rename<Old>To<New>On<Table>` | `RenameColumn` (не приймає аргументів `field:type` — колонка зберігає свій тип) |
| `CreateJoinTable<A>And<B>` | `CreateJoinTable` — обидві частини однюються в `a_b` (`loco-gen/src/migration.rs:54`), а шаблон далі множить цю назву для самої таблиці (`templates/migration/join_table.t:4`). Тож `CreateJoinTableUsersAndGroups` створює таблицю **`user_groups`** з reference-колонками `user` і `group` |
| усе інше | `Empty` |

```bash
# перейменовує movies.title на movies.name, з down(), що перейменовує назад
cargo loco generate migration RenameTitleToNameOnMovies
```

Складені назви працюють — `RenameFirstNameToGivenNameOnUserProfiles` — бо парсер прив'язується до останнього `On` і першого `To` перед ним, а не рахує слова.

:::caution
`Empty` — запасний варіант для назви, яку Loco не може прочитати, а його `up()` — це `todo!()` — запуск `cargo loco db migrate` запанікує, доки ви не напишете тіло. Це навмисно: заготовка, що мовчки «вдалося б», була б записана як застосована, і ваша схема назавжди розійшлася б із дійсністю. Генератор повідомляє про це під час створення.
:::

## Конвенції словозміни (`cruet` vs `heck`)

Задокументовано в `loco-gen/src/infer.rs:1-14`: **`cruet`** використовується *лише* для множення/однення (`to_plural`/`to_singular` — назви таблиць); **`heck`** — для *всіх* перетворень регістру (snake_case-колонки, PascalCase-назви сутностей/структур). Ці два крейти розходяться в написанні абревіатур/цифр (наприклад `i32`→`i_32` у `cruet` проти `i32` у `heck`; `HTTPServer`→`Httpserver` у `cruet` проти `HttpServer` у `heck`), тож змішування їх псує згенеровані ідентифікатори. Єдиний навмисний виняток: `guess_migration_type` нормалізує сире ім'я команди міграції snake-casing-ом `cruet` перед розбиттям на ключові частини, бо парсер налаштований саме під цю поведінку.

## Пов'язані довідкові сторінки

- [Ознаки (feature flags)](/uk/docs/reference/feature-flags/) — `with-db` та інші Cargo-ознаки, що обмежують генератори.
- Довідкові сторінки схеми/`ColType` DSL і пагінації запитів глибоко висвітлюють сторону автора міграцій (`add_column`, `add_reference`, `ColType`).
