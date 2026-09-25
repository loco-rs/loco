---
title: Довідник з CLI
description: Усі прапорці та підкоманди генератора застосунків `loco` і виконавчого CLI `cargo loco`.
sidebar:
  order: 2
---

Loco постачає **два** інтерфейси командного рядка:

| Бінарник | Крейт | Встановлення | Призначення |
|---|---|---|---|
| `loco` | `loco-new` (бінарний крейт, не `loco-rs`) | `cargo install loco` | Створює новий застосунок на диску. Один підкомандний: `new`. |
| `cargo loco` | генерується в кожен застосунок через `loco new`, спирається на `loco_rs::cli` | будується разом із вашим застосунком | Операції часу виконання над *вашим* застосунком: запуск сервера, міграції, генерація коду, виконання задач тощо. |

`cargo loco` має **дві реалізації `main()`**, які обираються Cargo-ознакою `with-db` (`src/cli.rs:712`, коли `with-db` увімкнено, та `src/cli.rs:872`, коли вимкнено). Збірка без `with-db` не має підкоманди `Db` і генераторів, що спираються на БД (`model`/`migration`/`scaffold`). Обидва варіанти спільно використовують глобальний прапорець верхнього рівня `-e, --environment <ENV>` (за замовчуванням `development`).

---

## 1. `loco new` — генератор застосунків

Джерело: `loco-new/src/bin/main.rs:30-62`.

### 1.1 Глобальний прапорець

| Прапорець | Тип | За замовчуванням | Призначення |
|---|---|---|---|
| `-l, --log <LEVEL>` | `LevelFilter` | `ERROR` | Багатослівність власного журналювання генератора (`main.rs:22-24`) |

### 1.2 Прапорці `new`

| Прапорець | Тип | За замовчуванням | Призначення |
|---|---|---|---|
| `-p, --path <PATH>` | `PathBuf` | `.` | Локальний каталог, у який генерувати (`main.rs:34-36`) |
| `-n, --name <NAME>` | `Option<String>` | немає — інтерактивне запитання | Назва застосунку (`main.rs:38-40`) |
| `--db <DB>` | `wizard::DBOption` | немає — інтерактивне запитання | Постачальник БД: `sqlite` \| `postgres` \| `none` (`main.rs:42-44`) |
| `--bg <BG>` | `wizard::BackgroundOption` | немає — інтерактивне запитання | Режим фонових робітників: `async` \| `queue-redis` \| `queue-postgres` \| `queue-sqlite` \| `blocking` (`main.rs:46-48`) |
| `--assets <ASSETS>` | `wizard::AssetsOption` | немає — інтерактивне запитання | Роздача статики: `serverside` \| `clientside` \| `none` (`main.rs:50-52`) |
| `--embedded-assets` | `bool` | `false` — інтерактивне запитання (лише в інтерактивному режимі, лише для serverside) | Вбудовує статичні ресурси в бінарник (ознака `embedded_assets`). Лише для serverside: з `--assets clientside` або `--assets none` прапорець **мовчки ігнорується**, бо `select_embedded_assets` повертає `false` для будь-якого несерверного вибору до того, як буде досягнуто перевірку, що відхиляє цю комбінацію (`wizard.rs:401-403`, `main.rs:121`, `settings.rs:118-123`). Його вказання (або повністю керований прапорцями запуск) пропускає інтерактивне запитання |
| `-a, --allow-in-git-repo` | `bool` | `false` | Пропускає перерву «ви всередині git-репозиторію, продовжити?» (`main.rs:54-56`) |
| `--os <OS>` | `wizard::OS` | `linux` на Unix, `windows` інакше | Генерує оптимізований під Unix чи Windows стартовий шаблон: `windows` \| `linux` \| `macos` (`main.rs:58-60`, `DEFAULT_OS` `main.rs:64-67`) |

Прапорця **`-t/--template`** немає, як і **`-v/--verbose`**. Перелік `Template` існує всередині та успадковує `ValueEnum`, але не прив'язаний до жодного аргументу CLI — вибір шаблону можливий лише інтерактивно (§1.4).

Якщо `--db`, `--bg` **та** `--assets` вказано всі, майстер пропускає всі запитання (`wizard.rs:291-297`) — включно з підтвердженням embedded-assets. Назву застосунку все одно буде запитано, якщо не передати `--name`, тож додавання `--name` зверху робить запуск повністю неінтерактивним.

### 1.3 Інтерактивні запитання (коли прапорці не вказано)

Джерело: `loco-new/src/wizard.rs`.

1. **Назва застосунку?** (за замовчуванням `myapp`) — непорожня, не починається з цифри, Unicode XID + `-`/`_` (`wizard.rs:201-267`).
2. **«Ви всередині git-репозиторію. Бажаєте продовжити?»** — лише якщо `cwd` є git-репозиторієм і `--allow-in-git-repo` не передано; за замовчуванням Ні, перериває у разі відмови (`wizard.rs:227-240`; `main.rs:91-94`).
3. **«Що ви хочете збудувати?»** — вибір шаблону (`wizard.rs:299-302`).
4. Умовні уточнення щодо БД / фону / статики залежно від обраного шаблону.

### 1.4 Шаблони

Джерело: `wizard.rs:12-27`, логіка розгалуження `wizard.rs:304-333`.

| Шаблон (перелік) | Мітка в меню | Запит про БД? | Запит про фон? | Статика |
|---|---|---|---|---|
| `SaasServerSideRendering` (за замовчуванням) | "Saas App with server side rendering" | так | так | примусово `Serverside` |
| `SaasClientSideRendering` | "Saas App with client side rendering" | так | так | примусово `Clientside` |
| `RestApi` | "Rest API (with DB and user auth)" | так | так | примусово `None` |
| `Lightweight` | "lightweight-service (minimal, only controllers and views)" | ні — примусово `None` | ні — примусово `Async` | примусово `None` |
| `Advanced` | "Advanced" | так | так | **запитує** (єдиний шаблон, що питає про конфігурацію статики) |

### 1.5 Переліки опцій (значення clap + мітки меню)

**`DBOption`** — `wizard.rs:39-79` — значення clap `--db`: `sqlite` (за замовчуванням), `postgres`, `none`.
| Значення | Шаблон рядка підключення | Примітки |
|---|---|---|
| `sqlite` | `sqlite://NAME_ENV.sqlite?mode=rwc` | за замовчуванням |
| `postgres` | `postgres://loco:loco@localhost:5432/NAME_ENV` | попереджає, що потрібен запущений екземпляр Postgres |
| `none` | — | `enable()` дорівнює `false`; вимикає генерацію БД, auth і mailer |

**`BackgroundOption`** — `wizard.rs:81-125` — значення clap `--bg`: `async` (за замовчуванням), `queue-redis`, `queue-postgres`, `queue-sqlite`, `blocking`.
| Значення | Мітка в меню | Примітки |
|---|---|---|
| `async` | "Async (in-process tokio async tasks)" | за замовчуванням |
| `queue-redis` | "Queue: Redis (standalone workers)" | попереджає, що обраний бекенд черги має бути досяжним; генерує з ознакою `worker_redis` |
| `queue-postgres` | "Queue: Postgres (standalone workers)" | попереджає, що обраний бекенд черги має бути досяжним; генерує з ознакою `worker` |
| `queue-sqlite` | "Queue: SQLite (standalone workers)" | попереджає, що обраний бекенд черги має бути досяжним; генерує з ознакою `worker` |
| `blocking` | "Blocking (run tasks in foreground)" | попереджає, що це **блокує запити**, доки задача не завершиться |

**`AssetsOption`** — `wizard.rs:127-162` — значення clap `--assets`: `serverside` (за замовчуванням), `clientside`, `none`.
| Значення | Мітка в меню | Примітки |
|---|---|---|
| `serverside` | "Server (configures server-rendered views)" | за замовчуванням |
| `clientside` | "Client (configures assets for frontend serving)" | виводить підказку: `cd frontend/ && npm install && npm run build` |
| `none` | "None" | — |

**`OS`** — `loco-new/src/lib.rs:41-51` — значення clap `--os`: `windows`, `linux`, `macos`. `windows` додає другий бінарний ціль `tool` у згенерований `Cargo.toml` (`Cargo.toml.t:65-70`).

### 1.6 Похідні налаштування генерації

Джерело: `loco-new/src/settings.rs:58-89`.

- БД увімкнено → `Features::default()` (ознаки loco-rs за замовчуванням застосовуються до згенерованого застосунку).
- БД **вимкнено** (шаблон Lightweight або `--db none`) → `default-features = false`, назви ознак = `["cli"]`; якщо фоновий режим — `queue-redis`, додається `"worker_redis"`; якщо фоновий режим — `queue-postgres` чи `queue-sqlite`, додається `"worker"`.
- Каркас `auth` та `mailer` увімкнено тоді й лише тоді, коли увімкнено БД.
- Вибір БД **дорого відкотити назад**: `--db none` видаляє крейт `migration`, модуль `models`, `AppContext::db` і два методи `Hooks`, доступні лише з `with-db`, і жоден генератор їх не повертає. Додавання бази даних згодом — ручна процедура, див. [Додати базу даних до наявного застосунку](/uk/docs/how-to/add-a-database/).
- Статика serverside → згенеровано `Initializers { view_engine: true }`.
- `loco_version_text`: зазвичай `version = "<LOCO_VERSION>"` (`loco-new/src/lib.rs:27`, зараз `1.1`); коли змінну середовища `LOCO_DEV_MODE_PATH` встановлено, стає `version = "*", path = "<той шлях>"` — так dogfood-ять локальну копію фреймворку.
- Видання (edition) самого згенерованого застосунку зафіксовано в `loco-new/base_template/Cargo.toml.t` незалежно від видання фреймворку `loco-rs`.

---

## 2. `cargo loco` — виконавчий CLI

Джерело: `src/cli.rs:64-171` (`enum Commands`).

### 2.1 Підкоманди верхнього рівня

| Підкоманда | Аліас | Обмежена ознакою | Прапорці | Призначення |
|---|---|---|---|---|
| `start` | `s` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `-a/--all`, `--scheduler`, `-b/--binding <ADDR>`, `-p/--port <PORT>`, `-n/--no-banner` (`worker`/`server_and_worker`/`all` взаємновиключні) | Запускає застосунок у певному режимі старту (`cli.rs:66-91`) |
| `db` | — | `#[cfg(feature = "with-db")]` | див. §2.2 | Операції з базою даних (`cli.rs:92-97`) |
| `routes` | — | — | немає | Виводить усі ендпоінти застосунку деревом (`cli.rs:98-99`) |
| `middleware` | — | — | `-c/--config` | Перелічує проміжні шари (спершу увімкнені, потім вимкнені); `--config` також виводить розв'язану конфігурацію кожного з них (`cli.rs:101-105`) |
| `task` | `t` | — | `[name]`, параметри `key:val...` | Виконує користувацьку задачу за назвою з параметрами `key:value` (`cli.rs:107-114`) |
| `jobs` | — | `#[cfg(feature = "worker")]` | див. §2.3 | Керує чергою фонових завдань (`cli.rs:115-120`) |
| `scheduler` | — | — | `-n/--name <NAME>`, `-t/--tag <TAG>`, `-c/--config <PATH>`, `-l/--list` | Запускає або інспектує планувальник (`cli.rs:121-137`) |
| `generate` | `g` | `#[cfg(debug_assertions)]` | див. §2.4 | Генерація коду (`cli.rs:138-146`) |
| `doctor` | — | — | `-c/--config`, `-p/--production` | Валідує/діагностує застосунок; `--config` натомість виводить розв'язану конфігурацію + середовище і пропускає перевірки. `--production` — **застарілий** і не є фільтром перевірок — він перемикає розв'язане *середовище* на `production` (з попередженням, що вказує на `--environment production`), тож перевірки виконуються проти продуктивної конфігурації. Які перевірки застосовуються, випливає з середовища (`cli.rs:152-157,756-767,874`) |
| `version` | — | — | немає | Виводить версію застосунку (`cli.rs:155-156`) |
| `watch` | `w` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `--scheduler` | Обгортає `cargo-watch -s 'cargo loco start ...'` (`cli.rs:158-170`, `838-867`); вимагає встановленого `cargo-watch` |

**Визначення режиму старту** (`cli.rs:736-750`): `--all` або (`--server-and-worker` **і** `--scheduler`) → `All`; лише `--server-and-worker` → `ServerAndWorker`; `--worker[=tags]` (+ `--scheduler`) → `WorkerAndScheduler` / `WorkerOnly`; лише `--scheduler` → `ServerAndScheduler`; нічого з переліченого → `ServerOnly`.

### 2.2 Підкоманди `db`

`enum DbCommands`, `src/cli.rs:483-523` — наявні лише коли увімкнено `with-db`.

| Команда | Прапорці | Призначення |
|---|---|---|
| `create` | — | Створює схему/базу даних |
| `migrate` | — | Застосовує всі невиконані висхідні міграції |
| `down` | `[steps]` (за замовчуванням `1`) | Відкочовує вказану кількість міграцій |
| `reset` | — | Видаляє всі таблиці, а потім повторно застосовує кожну міграцію |
| `status` | — | Показує стан міграцій |
| `entities` | — (`#[cfg(debug_assertions)]`) | Генерує файли сутностей `.rs` з поточної схеми БД |
| `truncate` | — | Очищає дані таблиць, не видаляючи самі таблиці |
| `seed` | `-r/--reset`, `-d/--dump`, `--dump-tables <csv>`, `--from <DIR>` (за замовчуванням `src/fixtures`) | Наповнює БД з файлів або вивантажує таблиці у файли (`cli.rs:505-520`) |
| `schema` | — | Виводить схему бази даних |

### 2.3 Підкоманди `jobs`

`enum JobsCommands`, `src/cli.rs:598-647` — наявні лише коли увімкнено ознаку `worker` (`worker_redis` неявно вмикає `worker`).

| Команда | Прапорці | Призначення |
|---|---|---|
| `cancel` | `--name <NAME>` (обов'язковий) | Позначає завдання, що відповідають `<NAME>`, як `cancelled` |
| `tidy` | — | Видаляє завдання в стані `completed` або `cancelled` |
| `purge` | `--max-age <DAYS>` (за замовчуванням `90`), `--status <csv>`, `--dump <PATH>` | Видаляє старі невдалі/скасовані завдання, за потреби спершу вивантаживши їх |
| `dump` | `--status <csv>`, `-f/--folder <DIR>` (за замовчуванням `.`) | Зберігає деталі завдань у файли |
| `import` | `-f/--file <PATH>` | Імпортує завдання з файлу |
| `requeue` | `--from-age <MINS>` (за замовчуванням `0`) | Повертає завдання в стані `processing`, старіші за вказаний вік, до `queued` |
| `retry` | `--id <ID>` (необов'язковий) | Повертає завдання в стані `failed` до `queued`. З `--id` — лише одне; без — усі |

`retry` і `requeue` — не той самий інструмент. `requeue` рятує завдання, які
зависли в `processing` після падіння робітника, і не може торкнутися завдання
в стані `failed`; `retry` — навпаки. У жодному драйвері черги немає
автоматичного повтору чи backoff, тож без `retry` невдале завдання є
термінальним.

У постачальника Redis повторно виконані завдання повертаються до черги
`default`: завдання не записує, до якої черги було надіслане, і єдиний слід
нього — наявність id у цій черзі — зникає, коли завдання падає. Команда
повідомляє про це, коли щось переміщує.

### 2.4 Підкоманди `generate`

`enum ComponentArg`, `src/cli.rs:173-382` — наявні лише у debug-збірках (`#[cfg(debug_assertions)]` на `Commands::Generate`). `model`/`migration`/`scaffold` додатково обмежені ознакою `#[cfg(feature = "with-db")]`. Повний синтаксис типів полів розглянуто в довіднику [Генератори та типи полів](/uk/docs/reference/generators/); ця таблиця показує лише форму CLI.

| Команда | Обмежена ознакою | Аргументи / прапорці | Примітки |
|---|---|---|---|
| `model` | `with-db` | `name`, `--without-tz`, `field:type ...` | Поля підтримують `references` (наприклад `director:references`) |
| `migration` | `with-db` | `name`, `--without-tz`, `field:type ...` | Додавання/видалення колонок, join-таблиці, порожні міграції, references |
| `scaffold` | `with-db` | `name`, `--without-tz`, `--no-auth`, `field:type ...` | Адаптивний — без прапорця виду: JSON API за замовчуванням, плюс типізовані React-хуки/сторінки, коли застосунок має `frontend/`. Обробники вимагають JWT, якщо не вказано `--no-auth`. `--api`/`--html`/`--htmx` приймаються для сумісності (див. нижче) |
| `controller` | — | `name`, `--auth`, `actions...` | Контролер JSON API — без прапорця виду. Публічний, якщо не вказано `--auth`. `--api`/`--html`/`--htmx` приймаються для сумісності (див. нижче) |
| `task` | — | `name` | |
| `scheduler` | — | немає | Створює шаблон конфігурації планувальника |
| `worker` | — | `name` | |
| `mailer` | — | `name` | |
| `data` | — | `name` | Завантажувач даних |
| `deployment` | — | `kind` = `docker` \| `nginx` \| `lambda` (`DeploymentKind`) | `docker` інспектує конфігурацію статичних ресурсів + `frontend/package.json`; `nginx` використовує налаштовані host/port; `lambda` створює точку входу AWS Lambda (`src/bin/lambda.rs`) і додає `lambda_http` |
| `override` | — | `[template_path]`, `--info` | Копіює вбудований шаблон генератора локально для кастомізації; без шляху перелічує всі доступні шаблони |

**Сумісність із вилученими прапорцями виду.** Адаптивна перебудова 1.0 вилучила прапорці `--api`/`--html`/`--htmx`/`-k/--kind` (і перелік `ScaffoldKind`). `scaffold`/`controller` досі *приймають* `--api` (нічого не робить — це headless-стандарт) та `--html`/`--htmx` (які повертають помилку з вказівкою на React SPA frontend, що замінив server-rendered views), тож домонові команди не падають із clap-помилкою `unexpected argument`.

---

## 3. `cargo loco --help` (перевірений вивід)

Довідка верхнього рівня, що точно відповідає `enum Commands`. Бінарник має назву `<module_name>-cli` (`examples/demo` будує `demo-cli`), і clap бере цю назву з `argv[0]`; рядок резюме над використанням — власний опис пакета `loco-rs`, вшитий там, де виводиться `#[command(about)]` (`src/cli.rs:44`), а не опис вашого застосунку:

```sh
The one-person framework for Rust

Usage: demo-cli [OPTIONS] <COMMAND>

Commands:
  start       Start an app
  db          Perform DB operations
  routes      Describe all application endpoints
  middleware  Describe all application middlewares
  task        Run a custom task
  jobs        Managing jobs queue
  scheduler   Run the scheduler
  generate    code generation creates a set of files and code templates based on a predefined set of rules
  doctor      Validate and diagnose configurations
  version     Display the app version
  watch       Watch and restart the app
  help        Print this message or the help of the given subcommand(s)

Options:
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
