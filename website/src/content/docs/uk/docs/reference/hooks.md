---
title: Трейт Hooks
description: "Повна поверхня трейту Hooks: кожен обов'язковий та наданий метод, його сигнатура і коли він виконується."
sidebar:
  order: 7
---

`Hooks` (`#[async_trait]`, `Send`, `src/app.rs:281-443`) — це єдиний трейт, який реалізує кожен Loco-застосунок — зазвичай на `struct App` у `src/app.rs` — щоб підключити маршрутизацію, workers, задачі, заповнення/очищення бази даних та колбеки життєвого циклу. `cargo loco generate` створює для вас заготовку `impl Hooks for App`; ця сторінка — вичерпний довідник того, що цей `impl` може і повинен містити.

## Обов'язкові методи

Реалізації за замовчуванням немає. Трейт не скомпілюється без них.

| Метод | Сигнатура | Призначення |
|---|---|---|
| `app_name` | `fn app_name() -> &'static str` (`:296`) | Повертає ім'я crate'а застосунку (за конвенцією `env!("CARGO_CRATE_NAME")`). |
| `boot` | `async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>` (`:323`) | Ініціалізує та запускає застосунок для заданих `StartMode` і `Environment`. Зазвичай делегує до `create_app::<Self, Migrator>(mode, environment, config)` (з БД) або `create_app::<Self>(mode, environment, config)` (без БД). |
| `routes` | `fn routes(_ctx: &AppContext) -> AppRoutes` (`:413`) | Визначає конфігурацію маршрутизації застосунку. |
| `connect_workers` | `async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>` (`:422`) | Реєструє workers фонових завдань у наданій `Queue`. |
| `register_tasks` | `fn register_tasks(tasks: &mut Tasks)` (`:425`) | Реєструє власні задачі `cargo loco task` у реєстрі `Tasks`. |
| `truncate` | `#[cfg(feature = "with-db")] async fn truncate(_ctx: &AppContext) -> Result<()>` (`:433`) | Очищає таблиці застосунку. Викликається, коли `config.database.dangerously_truncate` дорівнює `true`; корисно перед тестами. |
| `seed` | `#[cfg(feature = "with-db")] async fn seed(_ctx: &AppContext, path: &Path) -> Result<()>` (`:437`) | Заповнює базу даних початковими даними з `path`. |

`truncate` та `seed` існують у трейті лише коли ввімкнено Cargo feature `with-db`.

## Надані методи

Мають реалізацію за замовчуванням; перевизначайте, щоб змінити поведінку.

| Метод | Сигнатура | Поведінка за замовчуванням |
|---|---|---|
| `app_version` | `fn app_version() -> String` (`:285`) | Повертає `"dev".to_string()`. |
| `serve` | `async fn serve(app: AxumRouter, ctx: &AppContext, serve_params: &ServeParams) -> Result<()>` (`:331-351`) | Прив'язує `tokio::net::TcpListener` до `serve_params.binding:serve_params.port` і запускає `axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())` з graceful shutdown; при зупинці викликає `Self::on_shutdown(&ctx)`. |
| `init_logger` | `fn init_logger(_ctx: &AppContext) -> Result<bool>` (`:360-362`) | Повертає `Ok(false)`, тобто Loco ініціалізує власний стек tracing/logging. |
| `load_config` | `async fn load_config(env: &Environment) -> Result<Config>` (`:368-370`) | Повертає `env.load()` — стандартний шлях завантаження `config/{env}.yaml` (+ накладка `.local.yaml`). |
| `before_routes` | `async fn before_routes(_ctx: &AppContext) -> Result<AxumRouter<AppContext>>` (`:378`) | Повертає `Ok(AxumRouter::new())` — порожній роутер. |
| `after_routes` | `async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter>` (`:388`) | Повертає `Ok(router)` без змін. |
| `initializers` | `async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>>` (`:395`) | Повертає `Ok(vec![])` — без initializers. |
| `middlewares` | `fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>>` (`:401-403`) | Повертає `middleware::default_middleware_stack(ctx)`. |
| `before_run` | `async fn before_run(_app_context: &AppContext) -> Result<()>` (`:408`) | Повертає `Ok(())` — no-op. |
| `after_context` | `async fn after_context(ctx: AppContext) -> Result<AppContext>` (`:416`) | Повертає `Ok(ctx)` без змін. |
| `dump` | `#[cfg(feature = "with-db")] async fn dump(ctx: &AppContext, base: &Path) -> Result<()>` (`:593-596`) | Вивантажує кожну таблицю у YAML-фікстури під `base` за допомогою інтроспекції схеми (`db::dump_tables`). Парний метод до `seed`, лежить в основі `cargo loco db seed --dump`. Перевизначте його, щоб вивантажувати конкретні сутності через типізований, потоковий `db::dump::<users::ActiveModel>(..)` — для повної типізації та обмеженого споживання пам'яті. У трейті лише коли ввімкнено `with-db`. |
| `on_shutdown` | `async fn on_shutdown(_ctx: &AppContext)` (`:442`) | No-op. |

## Точки перевизначення

Нижче наведено найменш задокументовані частини `Hooks`. Кожен пункт точно описує, що змінює перевизначення.

### `init_logger` — власний стек tracing

```rust no-syntax-check="signature listing, no body by design"
fn init_logger(_ctx: &AppContext) -> Result<bool>
```

Виконується один раз під час завантаження, до того як решта контексту застосунку буде підключена. Повернення `Ok(true)` каже Loco **не** ініціалізувати власний логер — тоді застосунок сам відповідає за налаштування повного стеку tracing/logging. Повернення `Ok(false)` (за замовчуванням) залишає вбудований логер Loco на місці.

### `load_config` — заміна завантажувача конфігурації

```rust no-syntax-check="signature listing — no body, by design"
async fn load_config(env: &Environment) -> Result<Config>
```

Виконується під час завантаження, щоб створити `Config`, який передається у `boot`. За замовчуванням — `env.load()` (стандартне розрішення файлу `config/{env}.yaml`). Перевизначте, щоб завантажувати конфігурацію з іншого джерела (наприклад, віддаленого сервісу конфігурації), все ще повертаючи `Config`.

### `after_context` — пост-обробка `AppContext`

```rust no-syntax-check="signature listing — no body, by design"
async fn after_context(ctx: AppContext) -> Result<AppContext>
```

Виконується після того, як `AppContext` повністю побудовано (db, cache, storage, mailer, провайдер черги — все на місці), але до побудови маршрутів. Приймає `ctx` за значенням і має повернути (можливо, змінений) `AppContext` — єдиний хук, який дозволяє замінювати поля в самому контексті.

`AppContext` має атрибут `#[non_exhaustive]`, тому у своєму застосунку ви не можете написати `AppContext { storage, ..ctx }`. Використовуйте `ctx.into_builder()`, який переносить кожен компонент і дозволяє перевизначити потрібні:

```rust
async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(ctx
        .into_builder()
        .storage(Storage::single(drivers::local::new()).into())
        .build())
}
```

Додавання до `shared_store` взагалі не потребує перезбірки — він внутрішньо мутабельний, тож достатньо `ctx.shared_store.insert(my_service);` і потім `Ok(ctx)`.

### `before_run` — завантаження ресурсів перед запуском

```rust no-syntax-check="signature listing — no body, by design"
async fn before_run(_app_context: &AppContext) -> Result<()>
```

Виконується до того, як застосунок почне обслуговувати/працювати (стосується і сервера, і інших режимів запуску, як-от задачі/jobs, не лише HTTP serve). Використовуйте його, щоб завантажити або прогріти ресурси, які не мають бути на самому `AppContext`.

### `serve` — цикл HTTP-обслуговування

```rust no-syntax-check="signature listing — no body, by design"
async fn serve(app: AxumRouter, ctx: &AppContext, serve_params: &ServeParams) -> Result<()>
```

Виконується, коли застосунок запущено в режимі сервера. За замовчуванням прив'язує `TcpListener` і викликає `axum::serve` з `app.into_make_service_with_connect_info::<SocketAddr>()` — шар `connect_info` потрібен для витягування `remote_ip`/адреси клієнта в контролерах — загорнутий у graceful shutdown, який викликає `on_shutdown`. Перевизначайте лише щоб змінити механіку транспорту/обслуговування (наприклад, власне TLS-термінування); перевизначення без збереження `into_make_service_with_connect_info` зламає витягування connect-info.

### `app_version` — складена версійна стрічка

```rust no-syntax-check="signature listing — no body, by design"
fn app_version() -> String
```

Викликається всюди, де Loco повідомляє свою версію (наприклад, `cargo loco version`, діагностика на кшталт `/_ping`/`/_health`). За замовчуванням — літерал `"dev"`; перевизначте, щоб скласти справжню версійну стрічку, наприклад із `CARGO_PKG_VERSION` плюс git SHA.

## Примітка щодо сигнатури `boot`

Другий параметр `boot` — це `environment: &Environment` — посилання на enum `Environment`, **а не** `&str`:

```rust no-syntax-check="signature listing — no body, by design"
async fn boot(mode: StartMode, environment: &Environment, config: Config) -> Result<BootResult>
```

Деякі старіші доки та сніпети в обігу показують `environment: &str`; ця сигнатура застаріла. Приклади rustdoc на самому `boot` (`src/app.rs:448,455`) використовують `&Environment`, як і `src/controller/mod.rs:47`.
