---
title: Надсилаємо лист
description: Створіть мейлер генератором, напишіть шаблони та налаштуйте SMTP з правильним режимом TLS.
sidebar:
  order: 24
---

Мета: надіслати транзакційний лист (вітальне повідомлення, скидання пароля, сповіщення) з контролера або таска, не блокуючи запит, поки SMTP робить свою справу.

Мейлер доставляє лист через SMTP у фоні, використовуючи ту саму інфраструктуру [фонових воркерів](/uk/docs/how-to/add-worker/) — виклик мейлера ставить завдання `MailerWorker` у чергу (до черги `"mailer"`) і повертається одразу.

## 1. Створюємо мейлер

```sh
cargo loco generate mailer auth
```

Це створить `src/mailers/auth.rs`, додасть `pub mod auth;` до `src/mailers/mod.rs` і згенерує каталог шаблонів `welcome/`, а також `shared/`, що містить партіали, які може розширювати кожен мейлер:

```
src/
  mailers/
    auth/
      welcome/       <-- один каталог на лист, містить усі його частини
        subject.t
        html.t
        text.t
    shared/          <-- партіали розмітки, спільні для всіх мейлерів, пишуться один раз
      base.t
      subject.t
      text.t
    auth.rs          <-- визначення мейлера
```

Згенерований мейлер виглядає так:

```rust
#![allow(non_upper_case_globals)]
use loco_rs::prelude::*;
use serde_json::json;

static shared: Dir<'_> = include_dir!("src/mailers/shared");
static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");

#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {}
impl AuthMailer {
    pub async fn send_welcome(ctx: &AppContext, to: &str, msg: &str) -> Result<()> {
        Self::mail_template_with_shared(
            ctx,
            &welcome,
            &[&shared],
            mailer::Args {
                to: to.to_string(),
                locals: json!({
                  "message": msg,
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
```

Каталог шаблонів має містити рівно три файли — `subject.t`, `html.t`, `text.t` (усі — шаблони Tera, що рендеряться проти `locals`). Відсутність будь-якого з них — це помилка під час надсилання.

## 2. Викликаємо з контролера

```rust
use crate::mailers::auth::AuthMailer;

async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    // .. реєструємо користувача ..
    AuthMailer::send_welcome(&ctx, &user.email, "Welcome!").await?;
    format::json(())
}
```

`mail`/`mail_template` повертаються щойно завдання потрапить у чергу — фактична доставка через SMTP відбувається в процесі воркера.

## 3. Налаштовуємо SMTP

```yaml
# config/development.yaml — локальний ловець пошти (наприклад, mailtutan, MailHog)
mailer:
  smtp:
    enable: true
    host: localhost
    port: 1025
    secure: false
```

```yaml
# config/production.yaml — провайдер, що вимагає implicit TLS на порту 465
mailer:
  smtp:
    enable: true
    host: smtp.example.com
    port: 465
    tls: implicit # перекриває `secure` — дивіться нижче
    auth:
      user: postmaster@mg.example.com
      password: "<%= get_env(name='SMTP_PASSWORD') %>"
    hello_name: mail.example.com # необов'язковий ідентифікатор клієнта EHLO
```

### Обираємо правильний режим `tls`

`tls` — це основне налаштування; якщо воно задане, воно **перекриває** застарілий булів прапорець `secure`:

| Значення `tls` | Порт | Поведінка |
|---|---|---|
| `starttls` | 587 (типово) | З'єднання відкривається у відкритому тексті, потім оновлюється через `STARTTLS`. Саме це обирав (і досі обирає) `secure: true`. |
| `implicit` | 465 (типово) | З'єднання зашифроване з першого байта (SMTPS). **Обов'язково** для провайдерів, які приймають лише implicit TLS — `STARTTLS` не спрацює проти слухача на порту 465. |
| `none` | — | Відкритий текст, без TLS. Лише локальні приймачі (Mailpit, mailtutan). |

Якщо `tls` не задано, застаріле поле `secure` все одно працює: `secure: true` → `starttls`, `secure: false` → `none`. Якщо ви налаштовуєте провайдера, у документації якого зазначено «порт 465 / SMTPS», явно задайте `tls: implicit` — сам по собі `secure: true` цей режим не виражає.

## 4. Задаємо типову адресу відправника або пріоритет

Перекрийте `opts()` у своєму мейлері:

```rust
impl Mailer for AuthMailer {
    fn opts() -> MailerOpts {
        MailerOpts {
            from: "Acme <noreply@acme.example>".to_string(),
            reply_to: None,
            priority: 100, // типовий пріоритет фонової черги для завдань мейлера
        }
    }
}
```

Завдання мейлера ставляться в чергу з пріоритетом `100` за замовчуванням (`DEFAULT_MAILER_PRIORITY`) — що означає пріоритет у різних бекендах черги, дивіться у [Вибір бекенда черги](/uk/docs/how-to/choose-queue-backend/#черги-з-пріоритетами). Підніміть його, якщо листи конкретного мейлера (наприклад, скидання пароля) мають обганяти фонову роботу з нижчим пріоритетом.

## 5. CC, BCC та заголовки тредів

`Args` (передається в `mail_template`) і `Email` (передається в `mail`) обидва підтримують `cc`, `bcc` і поле `headers` для тредів:

```rust
Self::mail_template(
    ctx,
    &welcome,
    mailer::Args {
        to: user.email.clone(),
        cc: Some("audit@acme.example".to_string()),
        bcc: Some("archive@acme.example".to_string()),
        headers: Some(mailer::EmailHeaders {
            in_reply_to: Some(original_message_id.clone()),
            references: Some(original_message_id.clone()),
            message_id: None,
        }),
        locals: json!({ "name": user.name }),
        ..Default::default()
    },
)
.await?;
```

`EmailHeaders` відображається на `References`/`In-Reply-To`/`Message-ID` — це корисно, щоб групувати сповіщення в один тред у поштовому клієнті отримувача.

## 6. Запускаємо воркер мейлера

Доставка листів іде через інфраструктуру фонових воркерів, тож щоб щось фактично надсилалося, процес воркера має бути запущений:

```sh
cargo loco start --worker            # окремий процес воркера
cargo loco start --server-and-worker # сервер + воркер разом
```

## 7. Тестуємо без надсилання справжніх листів

Задайте `stub: true`, щоб перехоплювати листи замість їх надсилання:

```yaml
mailer:
  stub: true
```

Якщо пошта відправляється через воркер, задайте `workers.mode: ForegroundBlocking` у тестовій конфігурації, щоб надсилання завершувалося синхронно в межах тесту.

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        // .. викликаємо ендпоінт, що надсилає лист ..

        with_settings!({ filters => cleanup_email() }, {
            assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        });
    })
    .await;
}
```

`deliveries()` (доступне під фічєю `testing`) повідомляє, скільки листів було «надіслано» та який їхній вміст, тож ви можете перевіряти й те, й інше.

## Довідка

- Усі ключі YAML `mailer:`: [Довідник конфігурації](/uk/docs/reference/configuration/#mailer)
- Режими воркерів і запуск процесу воркера: [Додаємо фоновий воркер](/uk/docs/how-to/add-worker/)
