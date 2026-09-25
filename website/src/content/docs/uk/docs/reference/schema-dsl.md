---
title: DSL схеми та ColType
description: "Повний DSL міграційних схем: перелік типів колонок ColType, операції рівня таблиць, references/enums та 64-бітний авто-PK за замовчуванням."
sidebar:
  order: 9
---

`loco_rs::schema` — це Rails-подібний DSL, яким автори міграцій користуються — тонкий, ергономічний шар над `sea_query`/`SchemaManager` з Sea-ORM. До нього доходитися як `loco_rs::schema::*` (згенерований шаблон міграції робить `use loco_rs::schema::*;`), і він реекспортує весь `sea_orm_migration::schema::*` разом із власними доповненнями, тож примітиви нижчого рівня для визначення колонок (`string`, `integer`, `pk_auto`, …) доступні напряму, якщо `ColType` не покриває певний випадок.

Модуль обмежений ознакою `with-db` (`src/lib.rs:20-21`).

## 64-бітний авто-первинний ключ (стандарт 1.0)

`ColType::PkAuto` будує автоінкрементний **64-бітний** (`BIGINT`) первинний ключ, а не 32-бітний:

```rust no-syntax-check="a match arm quoted verbatim from src/schema.rs"
// src/schema.rs:330-332
Self::PkAuto => big_pk_auto(name),
```

`big_pk_auto` (реекспортований з `sea_orm_migration::schema`) — це `big_integer(name).auto_increment().primary_key()` — колонка `BigInteger`, тобто `i64` на боці Rust. Це навмисна зміна 1.0 порівняно з попереднім 32-бітним стандартом (коментар у `schema.rs:330-331`: Sea-ORM 2.0 відображає цілі числа SQLite на `i64`, а `BIGINT` PK — сучасний стандарт, як у Rails 5.1+).

Наслідок поширюється на зовнішні ключі: кожна FK-колонка, згенерована `create_table`/`create_join_table`/`add_reference`, має тип `ColType::BigInteger` (або `BigIntegerNull`, якщо nullable), щоб збігатися з колонкою `id`, на яку вказує (`schema.rs:677-683`, `schema.rs:781-782`). Альтернатива — `ColType::PkUuid` — первинний ключ `Uuid` без автоінкременту (`pk_uuid`, обгортає `uuid(name).primary_key()`).

| Варіант | Будує | Тип Rust | Anchor |
|---|---|---|---|
| `ColType::PkAuto` | `big_pk_auto(name)` — автоінкрементний `BIGINT` PK | `i64` | `schema.rs:332` |
| `ColType::PkUuid` | `pk_uuid(name)` — `UUID` PK, без автоінкременту | `Uuid` | `schema.rs:333` |

## `ColType` — перелік типів колонок

`enum ColType` (`schema.rs:161-284`) — це сторона-значення кожного кортежу `(name, ColType)`, що передається в `create_table`/`add_column`. Кожна родина нижче загалом слідує конвенції модифікаторів — але покриття не є рівномірним у межах родини (наприклад, `Boolean` не має `*Uniq`, `TimestampWithTimeZone` не має `*Uniq`, `Text` не має `*Len`):

- **(без суфікса)** — `NOT NULL`, без значення за замовчуванням, без обмеження унікальності.
- **`*Null`** — nullable.
- **`*Uniq`** — `NOT NULL` + унікальний індекс.
- **`*WithDefault(v)`** — `NOT NULL` + значення за замовчуванням.
- **`*Len(n)`** — фіксована/максимальна довжина, для родин `Char`/`String`/`Decimal`(precision, scale)/binary/varbit.

`ColType::to_def(&self, name) -> ColumnDef` (`schema.rs:328-470`) зіставляє кожен варіант із викликом конструктора `ColumnDef` Sea-ORM; таблиці нижче транскрибовані з того зіставлення плюс оголошення переліку.

### Первинні ключі — див. [вище](#64-бітний-авто-первинний-ключ-стандарт-10)

### Char / String / Text

| Варіант | Примітки |
|---|---|
| `Char`, `CharNull`, `CharUniq`, `CharWithDefault(char)` | колонка фіксованого одиночного символу |
| `CharLen(u32)`, `CharLenNull(u32)`, `CharLenUniq(u32)`, `CharLenWithDefault(u32, char)` | фіксована довжина `n` |
| `String`, `StringNull`, `StringUniq`, `StringWithDefault(String)` | рядок змінної довжини, без обмеження довжини |
| `StringLen(u32)`, `StringLenNull(u32)`, `StringLenUniq(u32)`, `StringLenWithDefault(u32, String)` | рядок змінної довжини з максимальною довжиною `n` |
| `Text`, `TextNull`, `TextUniq`, `TextWithDefault(String)` | необмежений текст; варіанта `Len` немає |

### Числові — цілі та беззнакові

| Варіант | Тип Rust | Примітки |
|---|---|---|
| `Integer`, `IntegerNull`, `IntegerUniq`, `IntegerWithDefault(i32)` | `i32` | 32-бітний знаковий |
| `SmallInteger`, `SmallIntegerNull`, `SmallIntegerUniq`, `SmallIntegerWithDefault(i16)` | `i16` | |
| `BigInteger`, `BigIntegerNull`, `BigIntegerUniq`, `BigIntegerWithDefault(i64)` | `i64` | також тип, автоматично згенерований для FK-колонок |
| `Unsigned`, `UnsignedNull`, `UnsignedUniq`, `UnsignedWithDefault(u32)` | `u32` | |
| `SmallUnsigned`, `SmallUnsignedNull`, `SmallUnsignedUniq`, `SmallUnsignedWithDefault(u16)` | `u16` | |
| `BigUnsigned`, `BigUnsignedNull`, `BigUnsignedUniq`, `BigUnsignedWithDefault(u64)` | `u64` | |

### Числові — decimal / float / money

| Варіант | Примітки |
|---|---|
| `Decimal`, `DecimalNull`, `DecimalUniq`, `DecimalWithDefault(f64)` | необмежена точність |
| `DecimalLen(u32, u32)`, `DecimalLenNull(u32, u32)`, `DecimalLenUniq(u32, u32)`, `DecimalLenWithDefault(u32, u32, f64)` | `(precision, scale)` |
| `Float`, `FloatNull`, `FloatUniq`, `FloatWithDefault(f32)` | `f32` |
| `Double`, `DoubleNull`, `DoubleUniq`, `DoubleWithDefault(f64)` | `f64` |
| `Money`, `MoneyNull`, `MoneyUniq`, `MoneyWithDefault(f64)` | колонка грошового типу |

### Boolean

| Варіант | Примітки |
|---|---|
| `Boolean`, `BooleanNull`, `BooleanWithDefault(bool)` | варіанта `*Uniq` немає |

### Дата / час

| Варіант | Примітки |
|---|---|
| `Date`, `DateNull`, `DateUniq`, `DateWithDefault(String)` | |
| `Time`, `TimeNull`, `TimeUniq`, `TimeWithDefault(String)` | |
| `DateTime`, `DateTimeNull`, `DateTimeUniq`, `DateTimeWithDefault(String)` | naive datetime, без часового поясу |
| `TimestampWithTimeZone`, `TimestampWithTimeZoneNull`, `TimestampWithTimeZoneWithDefault(String)` | з урахуванням часового поясу; варіанта `*Uniq` немає |
| `Interval(Option<PgInterval>, Option<u32>)`, `IntervalNull(..)`, `IntervalUniq(..)` | інтервал Postgres; аргументи — опціональний кваліфікатор поля `PgInterval` та опціональна точність |

### Binary

| Варіант | Примітки |
|---|---|
| `Binary`, `BinaryNull`, `BinaryUniq` | необмежений, варіанта зі значенням за замовчуванням немає |
| `BinaryLen(u32)`, `BinaryLenNull(u32)`, `BinaryLenUniq(u32)` | фіксована довжина `n` |
| `VarBinary(u32)`, `VarBinaryNull(u32)`, `VarBinaryUniq(u32)` | змінна довжина, максимум `n` |
| `Blob`, `BlobNull`, `BlobUniq` | |

### JSON

| Варіант | Примітки |
|---|---|
| `Json`, `JsonNull`, `JsonUniq` | JSON, що зберігається як текст |
| `JsonBinary`, `JsonBinaryNull`, `JsonBinaryUniq` | бінарний JSON (`jsonb` у Postgres) |

### UUID

| Варіант | Примітки |
|---|---|
| `Uuid`, `UuidNull`, `UuidUniq` | |
| `UuidWithDefault(String)`, `UuidUniqWithDefault(String)` | значення за замовчуванням — рядок сирого SQL-виразу, наприклад `"gen_random_uuid()"` (через `Expr::cust`) |

### Бітові рядки

| Варіант | Примітки |
|---|---|
| `VarBitLen(u32)`, `VarBitLenNull(u32)`, `VarBitLenUniq(u32)` | Postgres `VARBIT(n)` |

### Array

| Елемент | Сигнатура | Anchor |
|---|---|---|
| `ColType::Array(ColumnType)` / `ArrayNull(ColumnType)` / `ArrayUniq(ColumnType)` | обгортає `ColumnType` Sea-ORM як тип елемента | `schema.rs:276-278` |
| `ColType::array(kind: ArrayColType) -> Self` | будує `Array(..)` | `schema.rs:298-300` |
| `ColType::array_uniq(kind: ArrayColType) -> Self` | будує `ArrayUniq(..)` | `schema.rs:304-306` |
| `ColType::array_null(kind: ArrayColType) -> Self` | будує `ArrayNull(..)` | `schema.rs:310-312` |
| `enum ArrayColType { String, Int, BigInt, Float, Double, Bool }` | селектор типу елемента для конструкторів `array*` | `schema.rs:286-293` |

`array_col_type` відображає кожен `ArrayColType` на `sea_orm::ColumnType`: `String` → `ColumnType::string(None)`, `Int` → `Integer`, `BigInt` → `BigInteger`, `Float` → `Float`, `Double` → `Double`, `Bool` → `Boolean` (`schema.rs:314-323`).

### Enum

| Варіант | Примітки |
|---|---|
| `Enum(enum_name: String, variants: Vec<String>)` | `NOT NULL` |
| `EnumNull(enum_name, variants)` | nullable |
| `EnumWithDefault(enum_name, variants, default_value: String)` | `NOT NULL` + значення за замовчуванням |
| `EnumNullWithDefault(enum_name, variants, default_value: String)` | nullable + значення за замовчуванням |

(`schema.rs:280-283`)

Створення enum залежить від бекенда й обробляється автоматично в `create_table`/`create_join_table` (див. [Семантика типів enum за бекендом](#семантика-типів-enum-за-бекендом) нижче) — самі ви `CREATE TYPE` не викликаєте.

## Помічники визначення колонок (власні доповнення schema.rs)

Понад `ColType`, `schema.rs` визначає кілька окремих помічників, що використовуються для побудови сирих значень `ColumnDef`/`TableCreateStatement`/`TableAlterStatement`, поверх усього реекспортованого з `sea_orm_migration::schema`:

| Fn | Сигнатура | Поведінка | Anchor |
|---|---|---|---|
| `alter` | `fn alter<T: IntoIden + 'static>(name: T) -> TableAlterStatement` | `Table::alter().table(name)` | `schema.rs:19-21` |
| `table_auto_tz` | `fn table_auto_tz<T>(name: T) -> TableCreateStatement` | `Table::create().table(name).if_not_exists()` **з** уже доданими timestamptz-колонками `created_at`/`updated_at` (через `timestamps_tz`) | `schema.rs:24-29` |
| `timestamps_tz` | `fn timestamps_tz(t: TableCreateStatement) -> TableCreateStatement` | додає `created_at`/`updated_at` як колонки `timestamp_with_time_zone` зі значенням за замовчуванням `Expr::current_timestamp()` | `schema.rs:34-39` |
| `timestamptz` | `fn timestamptz<T>(name: T) -> ColumnDef` | timestamptz-колонка, не nullable | `schema.rs:53-61` |
| `timestamptz_null` | `fn timestamptz_null<T>(name: T) -> ColumnDef` | timestamptz-колонка, nullable | `schema.rs:42-50` |
| `enum_type` | `fn enum_type<T>(name: T, enum_name: &str) -> ColumnDef` | enum-колонка, не nullable | `schema.rs:64-72` |
| `enum_type_null` | `fn enum_type_null<T>(name: T, enum_name: &str) -> ColumnDef` | enum-колонка, nullable | `schema.rs:75-83` |
| `enum_type_with_default` | `fn enum_type_with_default<T>(name: T, enum_name: &str, default_value: &str) -> ColumnDef` | enum-колонка, не nullable + значення за замовчуванням | `schema.rs:93-102` |
| `enum_type_null_with_default` | `fn enum_type_null_with_default<T>(name: T, enum_name: &str, default_value: &str) -> ColumnDef` | enum-колонка, nullable + значення за замовчуванням | `schema.rs:112-121` |

`table_auto_tz` — це варіант з урахуванням часового поясу для `sea_orm_migration::schema::table_auto` (яка використовує naive, без tz, часові мітки) — `create_table`/`create_join_table` всередині використовують `table_auto_tz`, тож таблиці, побудовані через DSL, завжди отримують часові мітки `created_at`/`updated_at` з урахуванням часового поясу.

## Операції рівня таблиць

Усі — `async fn(m: &SchemaManager<'_>, ...) -> Result<(), DbErr>`, викликаються з `up`/`down` міграції.

| Fn | Сигнатура | Anchor |
|---|---|---|
| `create_table` | `create_table(m, table: &str, cols: &[(&str, ColType)], refs: &[(&str, &str)])` | `schema.rs:490-497` |
| `create_join_table` | `create_join_table(m, table, cols, refs)` — складений первинний ключ над reference-колонками | `schema.rs:512-519` |
| `create_table_without_timestamps` | `create_table_without_timestamps(m, table, cols, refs)` — без автоматичних `created_at`/`updated_at` | `schema.rs:537-544` |
| `create_join_table_without_timestamps` | `create_join_table_without_timestamps(m, table, cols, refs)` — join-таблиця, без часових міток | `schema.rs:559-566` |
| `add_column` | `add_column(m, table: &str, name: &str, atype: ColType)` | `schema.rs:721-735` |
| `remove_column` | `remove_column(m, table: &str, name: &str)` | `schema.rs:745-754` |
| `rename_column` | `rename_column(m, table: &str, from: &str, to: &str)` — перейменовує колонку, зберігаючи її тип і дані. Саме це генерує назва міграції `Rename<Old>To<New>On<Table>` (`migration/rename_column.t`) | `schema.rs:768-781` |
| `add_reference` | `add_reference(m, fromtbl: &str, totbl: &str, refname: &str)` | `schema.rs:764-839` |
| `remove_reference` | `remove_reference(m, fromtbl: &str, totbl: &str, refname: &str)` | `schema.rs:849-892` |
| `drop_table` | `drop_table(m, table: &str)` | `schema.rs:902-906` |
| `add_enum_values` | `add_enum_values(m, enum_name: &str, new_values: Vec<String>)` | `schema.rs:916-952` |
| `drop_enum_type` | `drop_enum_type(m, enum_name: &str)` | `schema.rs:962-987` |

Усі чотири функції `create_*` поділяють одну реалізацію (`create_table_impl`, `schema.rs:568-701`), параметризовану `is_join: bool` та `add_timestamps: bool`.

```rust
// schema.rs:474-497 (doc example)
create_table(m, "movies", vec![
    ("title", ColType::String)
], vec![]).await;
```
```sh
loco g migration CreateMovies title:string user:references
loco g migration CreateMovies title:string user:references:admin_id
```

### Параметри `cols` і `refs`

- `cols: &[(&str, ColType)]` — звичайні колонки, у порядку, кожна перетворюється на `ColumnDef` через `ColType::to_def`.
- `refs: &[(&str, &str)]` — по одному запису на кожен зовнішній ключ, який нова/змінена таблиця має нести. **Перший** елемент називає таблицю, *на яку* посилаються (вона однюється/приводиться до snake_case так само, як будь-яка назва таблиці); **другий** елемент — опціональна власна назва FK-колонки — передайте `""`, щоб використати стандартну `<однина(referenced_table)>_id` (обчислюється `reference_id`, `schema.rs:708-711`).
  - Додайте суфікс `?` до назви таблиці-посилання, щоб зробити FK-колонку **nullable**: `refs: &[("user?", "")]` — `create_table_impl` зрізає `?` перед нормалізацією назви таблиці (`schema.rs:664-669`).
  - Nullable-посилання отримують `ON DELETE SET NULL` / `ON UPDATE NO ACTION`; не-nullable — `ON DELETE CASCADE` / `ON UPDATE CASCADE` (`schema.rs:685-696`).
  - Згенерована FK-колонка завжди `ColType::BigInteger`/`BigIntegerNull` (відповідаючи 64-бітному стандарту `PkAuto`), якщо колонка з такою назвою вже не існує в `cols` (`schema.rs:676-683`).
  - Назва обмеження FK детермінована: `fk-{referenced_table}-{ref_column}-to-{table}` (`schema.rs:687`) — зверніть увагу, це **не** той самий порядок іменування, що використовують `add_reference`/`remove_reference` (див. наступний розділ): FK, доданий через параметр `refs` у `create_table`, називається `fk-users-user_id-to-movies`, тоді як `add_reference(m, "movies", "users", "")` називає його `fk-movies-user_id-to-users`. Виклик `remove_reference` проти FK, створеного через `refs` у `create_table` (а не через сам `add_reference`), шукатиме неправильну назву обмеження і не знайде його.

### `add_reference` / `remove_reference`

На відміну від кортежів `refs` вище, `add_reference`/`remove_reference` приймають назви таблиць у природному порядку «читається як» — `add_reference(m, "movies", "users", "")` читається як *«movies належить users»*: `fromtbl` — таблиця, що змінюється (`movies`), `totbl` — таблиця, на яку посилаються (`users`).

```rust
// schema.rs:756-764 (doc example)
add_reference(m, "movies", "users", "").await;
// ...
remove_reference(m, "movies", "users", "").await;
```

- `add_reference` завжди будує FK-колонку `ColType::BigInteger`, додає її через `ALTER TABLE ... ADD COLUMN` і — лише на MySQL/Postgres — також `ADD FOREIGN KEY` в тому самому операторі. На **SQLite він додає колонку, але пропускає обмеження FK** (SQLite не дозволяє додавати FK до наявної таблиці; за конвенцією Rails 5.2 це задокументований no-op — `schema.rs:817-830`). Будь-який інший бекенд повертає `DbErr::BackendNotSupported { ctx: "add_reference" }`.
- `remove_reference` видаляє назване обмеження FK на MySQL/Postgres; на **SQLite це no-op** з тієї ж причини (`schema.rs:879-883`). Будь-який інший бекенд повертає `DbErr::BackendNotSupported { ctx: "remove_reference" }`.

## Семантика типів enum за бекендом

`create_table_impl` сканує `cols` на наявність будь-якого варіанту `ColType::Enum*` і для кожного нового `enum_name`, який ще не зустрічався, перевіряє, чи тип уже існує (`check_enum_exists`, `schema.rs:124-159`, пошук `pg_type`, лише Postgres), перед створенням:

| Бекенд | Поведінка |
|---|---|
| Postgres | Створює нативний `CREATE TYPE ... AS ENUM (...)`, якщо він ще не існує. |
| SQLite | Нативного типу enum немає; колонка створюється як `TEXT` з поведінкою enum, що забезпечується визначенням колонки (без кроку `CREATE TYPE`). |
| MySQL | Не створюється як окремий тип; enum у MySQL інлайняться у визначення колонки. |
| інший | No-op. |

`add_enum_values(m, enum_name, new_values)` розширює наявний enum: на Postgres він виконує `ALTER TYPE {enum_name} ADD VALUE '{value}'` для кожного нового значення; на SQLite/MySQL це задокументований no-op (`schema.rs:916-952`). `drop_enum_type(m, enum_name)` виконує `DROP TYPE IF EXISTS {enum_name} CASCADE` на Postgres (захищений тією ж перевіркою існування) і є no-op деінде (`schema.rs:962-987`).

## Назвування таблиць

`normalize_table(table: &str) -> String` (`schema.rs:704-706`) множить і приводить до snake_case кожну назву таблиці, передану в DSL: `cruet::to_plural(table).to_snake_case()` — наприклад `"person"` → `"people"`, `"Movie"` → `"movies"`. Це застосовується до кожного аргументу-назви таблиці в `create_table`, `add_column`, `add_reference` тощо, тож викликачі передають однину чи множину, у будь-якому регістрі, і отримують ту саму нормалізовану таблицю.

## Часові мітки: стандарт vs `_without_timestamps`

`create_table`/`create_join_table` додають `created_at`/`updated_at` (через `table_auto_tz`), якщо ви не використовуєте варіант `_without_timestamps` (`create_table_without_timestamps`/`create_join_table_without_timestamps`), який будує голе `Table::create().if_not_exists()` без колонок часових міток — повний контроль над схемою.

CLI-прапорець генератора, що відповідає функціям `_without_timestamps`, — це **`--without-tz`** (не `--without-timestamps`):

```sh
loco g migration CreatePosts title:string --without-tz
loco g migration CreateJoinTableUsersAndGroups count:int --without-tz
loco g scaffold posts title:string! user:references --without-tz
```
(`src/cli.rs:193`, `:237`, `:241`, `:267`)

> Внутрішній doc-коментар у `schema.rs` (`schema.rs:533`, у `create_table_without_timestamps`) досі показує старе написання прапорця `--without-timestamps` у своєму прикладі — цей коментар застарілий; справжній CLI-прапорець, з'єднаний у `src/cli.rs`, — `--without-tz`.

## Пов'язане: відображення типів полів генератора

Скорочення типів полів `loco g model|migration|scaffold` (наприклад `title:string!`, `count:int^`, `user:references`) відображаються на ту саму поверхню `ColType` через `loco-gen/src/column.rs` (функція `parse_column` і перелік `ScalarType`). Примітно, що скорочення `int`/`unsigned` генератора також створюють 64-бітні колонки (`int` → `ColType::BigIntegerNull` / `Option<i64>`, `int!` → `ColType::BigInteger` / `i64`, родина `unsigned` → `BigUnsigned*` / `i64`) — узгоджено з 64-бітним стандартом `PkAuto` на цій сторінці. Повна таблиця типів полів (усі ~50 скорочень) належить на довідкову сторінку генераторів, на момент написання ще не опубліковану.
