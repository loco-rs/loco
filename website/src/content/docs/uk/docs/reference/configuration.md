---
title: Конфігурація
description: "Вичерпний довідник кожного YAML-ключа у файлі конфігурації застосунку Loco: пріоритет завантаження, визначення середовища та кожна підструктура конфігурації."
sidebar:
  order: 1
---

Ця сторінка — словник кожного ключа, який розуміє завантажувач конфігурації
Loco. Він документує `struct Config` (`src/config/mod.rs`) та його
підструктури (`src/config/{auth,server,database,logger,mailer,queue,cache}.rs`).
Для наративного огляду налаштувань і середовищ дивіться the-app/your-project.

## Завантаження та пріоритет

- Стандартна тека конфігурації: `config/` (`Config::new`, `src/config/mod.rs:128-131`). Перевизначається змінною середовища `LOCO_CONFIG_FOLDER` (читається в `Environment::load`, `src/environment.rs:59-64`).
- `Config::from_folder(env, path)` (`src/config/mod.rs:154-200`) завантажує **обидва** файли `{path}/{env}.yaml` та `{path}/{env}.local.yaml`, коли вони існують, і **глибоко зливає** їх, причому сторона `.local.yaml` перемагає ключ за ключем (`merge_yaml`, `src/config/mod.rs:243-257`). Злиття рекурсивно проходить відображення (мапи), тож локальному файлу достатньо повторити лише ті ключі, які він перевизначає; усе, що не є відображенням — скаляр або послідовність — замінюється целиком, а не поєднується. Коли існує лише один із двох файлів, використовується він сам. Завантажений шлях повідомляється як `"{base} (merged with {local})"`, коли було прочитано обидва.

  Якщо не існує жодного, завантаження зазнає невдачі з `Error::Message("no configuration file found in folder: ...")`.
- Перед розбором увесь YAML-файл рендериться як шаблон (`Config::load_yaml_value`, `src/config/mod.rs:211-217`; `src/config/template.rs`). Файли конфігурації використовують YAML-безпечні роздільники `<%= expr %>` / `<% stmt %>` / `<%# comment %>` — `<` не є індикаторним символом YAML, тож шаблоноване значення на кшталт `port: <%= get_env(name="PORT", default="5150") %>` залишається дійсним, неспотвореним YAML ще до того, як буде відрендерене. Під капотом вони перетворюються на рідні роздільники Tera `{{ }}`/`{% %}`/`{# #}` (`to_tera_syntax`, `src/config/template.rs:95-145`), а потім рендеряться через `tera::render_string` (`src/tera.rs:55-60`), який створює голий екземпляр Tera та реєструє на ньому власні функції Loco. Він не може використати `Tera::one_off`, бо той рендерить лише зі вбудованими засобами Tera — а `get_env(name=.., default=..)`, який використовується в усіх постачаних файлах конфігурації, **не** є одним із них: Tera 1 постачав його, Tera 2 прибрав, і Loco тепер реєструє власний (`src/tera.rs:16-32`). Рідні роздільники Tera `{{ }}`/`{% %}` досі рендеряться для зворотної сумісності, але вони застарілі (це синтаксис потокового відображення YAML, а не звичайний скаляр, тож YAML-форматувальник може переписати та зламати їх — дивіться [Модель конфігурації](/uk/docs/explanation/configuration-model/#спочатку-yaml--це-шаблон-а-вже-потім-конфігураційний-файл) і під час використання логують попередження.
- Невдачі розбору піднімають `Error::YAMLFile(err, path)` (`src/config/mod.rs:172-173`).
- `Config` реалізує `Display`, викидаючи сам себе назад у YAML (`src/config/mod.rs:191-196`).
- `Config::get_jwt_config(&self) -> Result<&JWT>` (`src/config/mod.rs:180-188`) повертає помилку, якщо `auth` чи `auth.jwt` відсутні.

### Визначення середовища

`environment::resolve_from_env()` (`src/environment.rs:32-38`) обирає активне ім'я середовища з таким пріоритетом:

1. `LOCO_ENV`
2. `RAILS_ENV`
3. `NODE_ENV`
4. резервний варіант: `"development"` (`DEFAULT_ENVIRONMENT`, `src/environment.rs:21`)

## Верхньорівневий `Config`

`struct Config` — `src/config/mod.rs:62-92`. Кожне поле є верхньорівневим YAML-ключем.

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `logger` | [`Logger`](#logger) | обов'язковий | `mod.rs:64` |
| `server` | [`Server`](#server) | обов'язковий | `mod.rs:65` |
| `database` | [`Database`](#database) | обов'язковий, лише коли увімкнена можливість `with-db` | `#[cfg(feature = "with-db")]`, `mod.rs:66-67` |
| `cache` | [`CacheConfig`](#cache) | опціональний — `#[serde(default)]`, стандарт `Null` | `mod.rs:68-69` |
| `queue` | `Option<`[`QueueConfig`](#queue)`>` | опціональний | `mod.rs:70` |
| `auth` | `Option<`[`Auth`](#auth)`>` | опціональний | `mod.rs:71` |
| `workers` | [`Workers`](#workers) | опціональний — `#[serde(default)]` | `mod.rs:72-73` |
| `mailer` | `Option<`[`Mailer`](#mailer)`>` | опціональний | `mod.rs:74` |
| `initializers` | `Option<Initializers>` (= `Option<BTreeMap<String, serde_json::Value>>`) | опціональний | `mod.rs:75`, псевдонім типу на `mod.rs:106` |
| `settings` | `Option<serde_json::Value>` | опціональний — `#[serde(default)]` | `mod.rs:88-89`; довільні налаштування застосунку, доступні через `ctx.config.settings` |
| `scheduler` | `Option<scheduler::Config>` | опціональний | `mod.rs:91`; структура належить області планувальника, тут не деталізується |

## `auth`

`struct Auth` — `src/config/auth.rs:13-17`.

```yaml
auth:
  jwt:
    location:              # optional, default: Bearer
      from: Bearer          # or: {from: Query, name: <string>} / {from: Cookie, name: <string>}
    secret: <base64 secret> # required — must be valid base64
    expiration: 604800      # required, u64 seconds (e.g. 7 days)
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `auth.jwt` | `Option<JWT>` | опціональний | `auth.rs:16` |
| `auth.jwt.location` | `Option<JWTLocationConfig>` | опціональний, стандарт: `Bearer` (розв'язується в `get_jwt_locations`, `src/controller/extractor/auth.rs:181-189`) | `auth.rs:24` |
| `auth.jwt.secret` | `String` | обов'язковий | `auth.rs:26`. **Має бути дійсним base64** — підписувач/верифікатор JWT викликають `EncodingKey`/`DecodingKey::from_base64_secret` (`src/auth/jwt.rs:83,108`); не-base64 рядок зазнає невдачі під час генерації/валідації токена, а не під час завантаження конфігурації |
| `auth.jwt.expiration` | `u64` (секунди) | обов'язковий | `auth.rs:28` |

`JWTLocationConfig` (`auth.rs:61-106`) приймає будь-яку з форм:
- `Single(JWTLocation)` — одне відображення розташування
- `Multiple(Vec<JWTLocation>)` — YAML-список відображень розташування, що пробуються по черзі, доки одне не дасть токен

`#[serde(untagged)]` застосовується лише до `Serialize`. `Deserialize` написаний вручну та диспетчеризується за формою вхідних даних — відображення є одним розташуванням, послідовність є резервним списком — тож некоректний запис повідомляє фактичну причину ( `from: cookie` у неправильному регістрі, `Cookie` без `name`) замість непрозорого "did not match any variant" нетегованого енуму.

`JWTLocation` (`#[serde(tag = "from")]`, `auth.rs:35-44`):

| Варіант | YAML-форма | Примітки |
|---|---|---|
| `Bearer` | `from: Bearer` | читає заголовок `Authorization: Bearer <token>` |
| `Query { name }` | `from: Query`<br>`name: <рядок>` | читає параметр рядка запиту |
| `Cookie { name }` | `from: Cookie`<br>`name: <рядок>` | читає cookie |

Пов'язане, але не налаштовуване через YAML: стандартний алгоритм підпису — **HS512** (`JWT_ALGORITHM`, `src/auth/jwt.rs:13`), що перевизначається в коді через `JWT::algorithm(..)` (`jwt.rs:51`), а не через конфігурацію.

## `server`

`struct Server` — `src/config/server.rs:29-45`.

```yaml
server:
  binding: localhost     # optional, default "localhost"
  port: 5150             # required
  host: http://localhost # required
  ident: <string>        # optional — overrides the `Server` response header
  middlewares: {}        # optional, default {} — see the middleware catalog reference
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `server.binding` | `String` | опціональний — `#[serde(default = "default_binding")]` → `"localhost"` | `server.rs:33-34,47-49` |
| `server.port` | `i32` | обов'язковий | `server.rs:36` |
| `server.host` | `String` | обов'язковий | `server.rs:38` |
| `server.ident` | `Option<String>` | опціональний | `server.rs:40`. Коли задано, замінює значення заголовка `Server` |
| `server.middlewares` | `middleware::Config` | опціональний — `#[serde(default)]` | `server.rs:44`. Структура належить області middleware; дивіться довідник каталогу middleware для ключів кожного middleware |

`Server::full_url() -> String` повертає `"{host}:{port}"` (`server.rs:52-55`).

## `workers`

`struct Workers` — `src/config/server.rs:64-68`.

```yaml
workers:
  mode: BackgroundQueue  # optional, default BackgroundQueue
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `workers.mode` | `WorkerMode` | опціональний — `Workers` виводить `Default` | `server.rs:67` |

`WorkerMode` (`server.rs:71-83`):

| Варіант | Стандарт? | Примітки |
|---|---|---|
| `BackgroundQueue` | так | Воркери працюють асинхронно через бекенд черги. **Потребує налаштованого `queue`** |
| `ForegroundBlocking` | ні | Воркери працюють у процесі та блокують викликача, доки задача не завершиться |
| `BackgroundAsync` | ні | Воркери працюють асинхронно в процесі (async-задача, без зовнішньої черги) |

## `database`

`struct Database` — `src/config/database.rs:22-84`. Присутній/обов'язковий лише коли увімкнена можливість `with-db`.

```yaml
database:
  uri: postgres://root:12341234@localhost:5432/myapp_development  # required
  enable_logging: true       # required — SQLx statement logging
  min_connections: 1         # required
  max_connections: 1         # required
  connect_timeout: 500       # required, milliseconds
  idle_timeout: 500          # required, milliseconds
  acquire_timeout: 500       # optional, milliseconds
  auto_migrate: true         # optional, default false
  dangerously_truncate: false# optional, default false
  dangerously_recreate: false# optional, default false
  run_on_start: <sql/pragma> # optional
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `database.uri` | `String` | обов'язковий | `database.rs:28`. Напр. `postgres://...` чи `sqlite://db.sqlite?mode=rwc` |
| `database.enable_logging` | `bool` | обов'язковий | `database.rs:31` — увімкнює логування інструкцій SQLx |
| `database.min_connections` | `u32` | обов'язковий | `database.rs:34` |
| `database.max_connections` | `u32` | обов'язковий | `database.rs:37` |
| `database.connect_timeout` | `u64` (мс) | обов'язковий | `database.rs:40` |
| `database.idle_timeout` | `u64` (мс) | обов'язковий | `database.rs:43` |
| `database.acquire_timeout` | `Option<u64>` (мс) | опціональний | `database.rs:46` |
| `database.auto_migrate` | `bool` | опціональний — `#[serde(default)]` | `database.rs:51-52`. Запускає невиконані міграції під час завантаження; рекомендовано для розробки, не рекомендовано в продакшені |
| `database.dangerously_truncate` | `bool` | опціональний — `#[serde(default)]` | `database.rs:56-57`. Видаляє дані рядків під час завантаження; типово використовується в `test` |
| `database.dangerously_recreate` | `bool` | опціональний — `#[serde(default)]` | `database.rs:63-64`. Викидає та створює схему заново під час завантаження |
| `database.run_on_start` | `Option<String>` | опціональний | `database.rs:83`. Довільні інструкції SQL/PRAGMA, що виконуються після встановлення з'єднання. Для SQLite, якщо не задано, Loco застосовує власні стандарти PRAGMA (`foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=134217728`, `journal_size_limit=67108864`, `cache_size=2000`, `busy_timeout=5000`) |

Примітка: допоміжні функції `db_min_conn()=1` / `db_max_conn()=20` / `db_connect_timeout()=500` / `db_idle_timeout()=500` у `database.rs:86-100` — це **не** стандарти для самого `Database` (чиї числові поля не мають `#[serde(default)]` і є обов'язковими) — вони повторно використовуються як значення `#[serde(default = ...)]` для конфігурацій [`queue`](#queue) Postgres/Sqlite нижче.

## `logger`

`struct Logger` — `src/config/logger.rs:21-49`.

```yaml
logger:
  enable: true            # required
  pretty_backtrace: false # optional, default false
  level: debug            # required — off|trace|debug|info|warn|error
  format: compact         # required — compact|pretty|json
  override_filter: <str>  # optional — EnvFilter directive string
  file_appender:          # optional
    enable: true           # required within block
    non_blocking: false    # optional, default false, within block
    level: info            # required within block
    format: json           # required within block
    rotation: daily        # required within block — minutely|hourly|daily|never
    dir: ./logs            # optional, default "./logs"
    filename_prefix: <s>   # optional
    filename_suffix: <s>   # optional
    max_log_files: 7       # required within block
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `logger.enable` | `bool` | обов'язковий | `logger.rs:24` |
| `logger.pretty_backtrace` | `bool` | опціональний — `#[serde(default)]` | `logger.rs:28-29`. Коли `true`, примусово вмикає гарно відформатовані backtrace (зручно для розробки); вимикайте в чутливих до продуктивності продакшен-розгортаннях |
| `logger.level` | `logger::LogLevel` | обов'язковий | `logger.rs:34`. Варіанти: `off`, `trace`, `debug`, `info` (власний `#[default]` енуму, але саме поле не має `#[serde(default)]`, тож має бути присутнім у YAML), `warn`, `error` (`src/logger.rs:15-36`) |
| `logger.format` | `logger::Format` | обов'язковий | `logger.rs:39`. Варіанти: `compact` (`#[default]`), `pretty`, `json` (`src/logger.rs:39-48`) |
| `logger.override_filter` | `Option<String>` | опціональний | `logger.rs:45`. Рядок директиви `EnvFilter` з `tracing-subscriber` |
| `logger.file_appender` | `Option<LoggerFileAppender>` | опціональний | `logger.rs:48` |
| `logger.file_appender.enable` | `bool` | обов'язковий у блоці | `logger.rs:54` |
| `logger.file_appender.non_blocking` | `bool` | опціональний — `#[serde(default)]` | `logger.rs:57-58` |
| `logger.file_appender.level` | `logger::LogLevel` | обов'язковий у блоці | `logger.rs:63` |
| `logger.file_appender.format` | `logger::Format` | обов'язковий у блоці | `logger.rs:68` |
| `logger.file_appender.rotation` | `logger::Rotation` | обов'язковий у блоці | `logger.rs:71`. Варіанти: `minutely`, `hourly` (`#[default]`), `daily`, `never` (`src/logger.rs:51-62`) |
| `logger.file_appender.dir` | `Option<String>` | опціональний, стандарт `"./logs"`, коли не задано (застосовується під час ініціалізації file appender, `src/logger.rs:113`) | `logger.rs:76` |
| `logger.file_appender.filename_prefix` | `Option<String>` | опціональний | `logger.rs:79` |
| `logger.file_appender.filename_suffix` | `Option<String>` | опціональний | `logger.rs:82` |
| `logger.file_appender.max_log_files` | `usize` | обов'язковий у блоці | `logger.rs:85` |

## `mailer`

`struct Mailer` — `src/config/mailer.rs:29-35`.

```yaml
# development: capture instead of sending
mailer:
  stub: false          # optional, default false
  smtp:
    enable: true       # required
    host: localhost    # required
    port: 1025         # required
    secure: false      # required — legacy shorthand, see below

# production: implicit TLS on port 465 (SMTPS)
mailer:
  smtp:
    enable: true
    host: smtp.example.com
    port: 465
    tls: implicit       # overrides `secure`; see below
    auth:
      user: postmaster@mg.example.com
      password: "<%= get_env(name='SMTP_PASSWORD') %>"
    hello_name: <string> # optional — EHLO client id
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `mailer.stub` | `bool` | опціональний — `#[serde(default)]` | `mailer.rs:33-34`. Коли `true`, пошта перехоплюється, а не надсилається |
| `mailer.smtp` | `Option<SmtpMailer>` | опціональний | `mailer.rs:31` |
| `mailer.smtp.enable` | `bool` | обов'язковий | `mailer.rs:55` |
| `mailer.smtp.host` | `String` | обов'язковий | `mailer.rs:57` |
| `mailer.smtp.port` | `u16` | обов'язковий | `mailer.rs:59` |
| `mailer.smtp.secure` | `bool` | обов'язковий | `mailer.rs:65`. Застаріле скорочення: `true` обирає `STARTTLS` (порт 587), `false` обирає відкритий текст |
| `mailer.smtp.tls` | `Option<MailerTls>` | опціональний — `#[serde(default)]` | `mailer.rs:69`. **Коли задано, перевизначає `secure`.** Варіанти (`#[serde(rename_all = "lowercase")]`, `mailer.rs:38-50`): `starttls` (опортуністичний TLS, порт 587 — те, що обирає `secure: true`), `implicit` (TLS з першого байта, SMTPS, порт 465 — потрібно провайдерам, що приймають лише неявний TLS), `none` (відкритий текст) |
| `mailer.smtp.auth` | `Option<MailerAuth>` | опціональний | `mailer.rs:71` |
| `mailer.smtp.auth.user` | `String` | обов'язковий у блоці | `mailer.rs:93` |
| `mailer.smtp.auth.password` | `String` | обов'язковий у блоці | `mailer.rs:95` |
| `mailer.smtp.hello_name` | `Option<String>` | опціональний | `mailer.rs:73`. Ідентифікатор клієнта `EHLO`, що надсилається замість імені хоста |

Ефективний режим TLS розв'язується в `SmtpMailer::tls_mode()` (`mailer.rs:76-87`): якщо задано `tls`, він перемагає беззастережно; інакше `secure: true` → `Starttls`, `secure: false` → `None`.
## `queue`

`enum QueueConfig` — `src/config/queue.rs:5-14`, `#[serde(tag = "kind")]` з варіантами `Redis`, `Postgres`, `Sqlite`.

```yaml
# kind: Redis
queue:
  kind: Redis
  uri: redis://127.0.0.1                # required
  dangerously_flush: false              # optional, default false
  queues: [high, low]                   # optional — priority order, first = most important
  num_workers: 2                        # optional, default 2
  # reaper:                             # optional, disabled by default (opt-in)
  #   age_minutes: 10                   # requeue jobs stuck in `processing` for longer than this
  #   interval_seconds: 60              # optional, default 60 — how often to sweep

# kind: Postgres
queue:
  kind: Postgres
  uri: postgres://...                   # required
  dangerously_flush: false              # optional, default false
  enable_logging: false                 # optional, default false
  max_connections: 20                   # optional, default 20
  min_connections: 1                    # optional, default 1
  connect_timeout: 500                  # optional, default 500 (ms)
  idle_timeout: 500                     # optional, default 500 (ms)
  poll_interval_sec: 1                  # optional, default 1
  num_workers: 2                        # optional, default 2
  # reaper:                             # optional, disabled by default (opt-in)
  #   age_minutes: 10                   # requeue jobs stuck in `processing` for longer than this
  #   interval_seconds: 60              # optional, default 60 — how often to sweep

# kind: Sqlite (same shape as Postgres)
queue:
  kind: Sqlite
  uri: sqlite://...
  poll_interval_sec: 1                  # optional, default 1 (own default fn)
  # ...remaining keys identical to Postgres, including the optional `reaper`
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `queue.kind` | тег: `Redis` \| `Postgres` \| `Sqlite` | обов'язковий | `queue.rs:6-14` |
| **Redis** (`RedisQueueConfig`, `queue.rs:16-28`) | | | |
| `queue.uri` | `String` | обов'язковий | `queue.rs:18` |
| `queue.dangerously_flush` | `bool` | опціональний — `#[serde(default)]` | `queue.rs:20` |
| `queue.queues` | `Option<Vec<String>>` | опціональний | `queue.rs:24`. Оголошує іменовані черги з пріоритетами; перший запис — найважливіший |
| `queue.num_workers` | `u32` | опціональний, стандарт `2` (`num_workers()`) | `queue.rs:26-27` |
| `queue.reaper` | `Option<ReaperConfig>` | опціональний, стандарт `None` (вимкнений) | `queue.rs:29-31`. Дивіться нижче |
| **Postgres** (`PostgresQueueConfig`, `queue.rs:30-57`) | | | |
| `queue.uri` | `String` | обов'язковий | `queue.rs:32` |
| `queue.dangerously_flush` | `bool` | опціональний, стандарт `false` | `queue.rs:34-35` |
| `queue.enable_logging` | `bool` | опціональний, стандарт `false` | `queue.rs:37-38` |
| `queue.max_connections` | `u32` | опціональний, стандарт `20` (`db_max_conn()`) | `queue.rs:40-41` |
| `queue.min_connections` | `u32` | опціональний, стандарт `1` (`db_min_conn()`) | `queue.rs:43-44` |
| `queue.connect_timeout` | `u64` (мс) | опціональний, стандарт `500` (`db_connect_timeout()`) | `queue.rs:46-47` |
| `queue.idle_timeout` | `u64` (мс) | опціональний, стандарт `500` (`db_idle_timeout()`) | `queue.rs:49-50` |
| `queue.poll_interval_sec` | `u32` | опціональний, стандарт `1` (`pgq_poll_interval()`) | `queue.rs:52-53` |
| `queue.num_workers` | `u32` | опціональний, стандарт `2` | `queue.rs:55-56` |
| `queue.reaper` | `Option<ReaperConfig>` | опціональний, стандарт `None` (вимкнений) | `queue.rs:57-59`. Дивіться нижче |
| **Sqlite** (`SqliteQueueConfig`, `queue.rs:59-86`) | | | |
| — | ідентичні поля до Postgres, включно з `queue.reaper` | | `poll_interval_sec` отримує стандарт через власну `sqlt_poll_interval()=1` (`queue.rs:81-82,92-94`); усі інші стандарти спільні з Postgres через ті самі допоміжні функції `db_*` |
| **`ReaperConfig`** (`queue.rs`, усі три бекенди) | | | Опціональний reaper часу очікування видимості: коли задано, постачальник черги створює фонову задачу, яка періодично повертає в чергу задачі, що застрягли в `processing` (напр. після падіння воркера), повторно використовуючи ту саму логіку, що й `cargo loco jobs requeue`. Якщо його не задати, зберігається попередня поведінка — без автоматичного повернення в чергу. |
| `queue.reaper.age_minutes` | `i64` | обов'язковий (лише якщо задано `reaper`) | Повертає в чергу задачі, що перебувають у `processing` довше за цю кількість хвилин |
| `queue.reaper.interval_seconds` | `u64` | опціональний, стандарт `60` (`default_reaper_interval_seconds()`) | Як часто reaper перевіряє застарілі задачі |

## `cache`

`enum CacheConfig` — `src/config/cache.rs:4-16`, `#[serde(tag = "kind")]`. **Стандартний варіант: `Null`** (`#[default]`, `cache.rs:14-15`) — це те, що виробляє `#[serde(default)]` поля `Config.cache`, коли ключ `cache` повністю опущено.

```yaml
cache:
  kind: InMem              # requires the `cache_inmem` feature
  max_capacity: 33554432   # optional, default 33554432 bytes (32 MiB)

# --- or ---
cache:
  kind: Redis              # requires the `cache_redis` feature
  uri: redis://...         # required
  max_size: 100            # required — max pool connections

# --- or (default) ---
cache:
  kind: "Null"             # no-op cache; used when `cache` key is omitted
                           # must be quoted — bare Null is YAML's null, not the string
```

| Ключ | Тип | Обов'язковий? | Примітки |
|---|---|---|---|
| `cache.kind` | тег: `InMem` \| `Redis` \| `"Null"` | обов'язковий, якщо `cache` присутній | `cache.rs:6-16`. `Null` треба писати в лапках: без лапок YAML розв'язує його в null, і тегований енум зазнає невдачі десеріалізації |
| **InMem** (`InMemCacheConfig`, `cache.rs:18-22`) — обмежений можливістю `cache_inmem` | | | |
| `cache.max_capacity` | `u64` | опціональний, стандарт `33554432` (`32 * 1024 * 1024`, `cache_in_mem_max_capacity()`) | `cache.rs:20-21,24-26` |
| **Redis** (`RedisCacheConfig`, `cache.rs:28-33`) — обмежений можливістю `cache_redis` | | | |
| `cache.uri` | `String` | обов'язковий | `cache.rs:30` |
| `cache.max_size` | `u32` | обов'язковий — максимум з'єднань пулу | `cache.rs:32` |
| **Null** | (без полів) | — | стандартний no-op кеш |

Якщо відповідна можливість (`cache_inmem` / `cache_redis`) не скомпільована, те значення `kind` зазнає невдачі десеріалізації.

## `initializers`, `settings`, `scheduler`

- `initializers: Option<BTreeMap<String, serde_json::Value>>` (`mod.rs:75,106`) — довільне відображення, що споживається ініціалізаторами застосунку (напр. ініціалізатором `oauth2`, що читає `initializers.oauth2`). Ключі та форми визначаються тим ініціалізатором, який їх читає, а не самим `Config`.
- `settings: Option<serde_json::Value>` (`mod.rs:88-89`) — довільні визначені застосунком налаштування; десеріалізуйте власний тип із `ctx.config.settings`.
- `scheduler: Option<scheduler::Config>` (`mod.rs:91`) — структура та ключі належать області планувальника; на цій сторінці не деталізуються.

## Змінні середовища

| Змінна | Призначення | Джерело |
|---|---|---|
| `LOCO_ENV` | Обирає активне середовище; найвищий пріоритет | `src/environment.rs:22,34` |
| `RAILS_ENV` | Резервний варіант, якщо `LOCO_ENV` не задано | `src/environment.rs:23,35` |
| `NODE_ENV` | Резервний варіант, якщо обидва вище не задано | `src/environment.rs:24,36` |
| `LOCO_CONFIG_FOLDER` | Перевизначає теку `config/`, з якої завантажує Loco | `src/env_vars.rs:16`, читається в `Environment::load` (`src/environment.rs:59-64`) |
| `LOCO_DATA` | Шлях до теки даних | `src/env_vars.rs:20` |
| `LOCO_POSTGRES_DB_OPTIONS` | Додаткові опції з'єднання Postgres (мають сенс лише з `with-db`) | `src/env_vars.rs:8` |
| `SCHEDULER_CONFIG` | Шлях до файлу конфігурації планувальника | `src/env_vars.rs:18` |
| `RUST_BACKTRACE` | Фактично примусово `1`, коли `logger.pretty_backtrace: true` | ініціалізація логера |
| будь-яке ім'я, передане в `get_env(name=.., default=..)` у YAML-файлі конфігурації | Впроваджується у відрендерений YAML під час завантаження власною Tera-функцією `get_env` у Loco | `src/tera.rs:16-32` |

Секрети (JWT `secret`, SMTP `password`, облікові дані `uri` бази даних) — це звичайні поля `String` без виділеного типу сховища; конвенція полягає в тому, щоб впроваджувати їх через `<%= get_env(name="...") %>` під час завантаження конфігурації, а не зашивати їх у YAML-файл.
