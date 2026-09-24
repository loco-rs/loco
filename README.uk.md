 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Ласкаво просимо до Loco</h1>

   <h3>
🚂 Loco — це Rust on Rails.
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md) · [Bahasa Indonesia](./README.id.md) · Українська

## Що таке Loco?

`Loco` значною мірою натхненний Rails. Якщо ви знаєте Rails і Rust, то почуватиметеся як удома. Якщо ж ви знайомі лише з Rails і тільки починаєте вивчати Rust, Loco стане для вас приємним відкриттям. Попереднє знання Rails не є обов’язковим.

Щоб докладніше дізнатися, як працює Loco, ознайомтеся з посібниками, прикладами та довідником API на нашому [сайті документації](https://loco.rs).

## Можливості Loco

* `Домовленості замість конфігурації:` Як і Ruby on Rails, Loco робить акцент на простоті та продуктивності, зменшуючи потребу в шаблонному коді. Завдяки продуманим типовим налаштуванням розробники можуть зосередитися на бізнес-логіці, а не витрачати час на конфігурацію.

* `Швидка розробка:` Loco створено для продуктивної роботи розробників: менше шаблонного коду та інтуїтивно зрозумілі API дають змогу швидко вдосконалювати застосунки й створювати прототипи з мінімальними зусиллями.

* `Інтеграція з ORM:` Моделюйте предметну область за допомогою сутностей без потреби писати SQL. Визначайте зв’язки, перевірку даних і власну логіку безпосередньо в сутностях, щоб полегшити підтримку та масштабування.

* `Контролери:` Опрацьовуйте параметри й тіла вебзапитів, перевіряйте дані та формуйте відповіді з урахуванням типу вмісту. Ми використовуємо Axum завдяки його швидкодії, простоті та розширюваності. Контролери також дають змогу легко створювати проміжні обробники (middleware), щоб додавати автентифікацію, журналювання чи обробку помилок перед передаванням запитів до основних дій контролера.

* `Представлення:` Loco можна інтегрувати з рушіями шаблонів для створення динамічного HTML-вмісту.

* `Фонові завдання:` Виконуйте завдання з великим обсягом обчислень або операцій введення-виведення у фоновому режимі за допомогою черги на основі Redis або потоків. Щоб створити фоновий обробник, достатньо реалізувати функцію `perform` трейту `Worker`.

* `Планувальник:` Спрощує роботу з традиційною, часто громіздкою системою crontab і дає змогу зручно планувати виконання завдань або скриптів оболонки.

* `Надсилання пошти:` Поштовий компонент надсилає електронні листи у фоновому режимі через наявну інфраструктуру фонових обробників Loco. Усе це працює без зайвих зусиль з вашого боку.

* `Сховище:` Loco Storage спрощує роботу з файлами та надає різноманітні операції над ними. Дані можна зберігати в пам’яті, на диску або в хмарних сервісах, як-от AWS S3, GCP та Azure.

* `Кеш:` Loco надає рівень кешування для підвищення швидкодії застосунку завдяки збереженню даних, до яких часто звертаються.

Більше про можливості Loco читайте на нашому [сайті документації](https://loco.rs/docs/getting-started/tour/).

## Початок роботи
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Тепер можна створити новий застосунок (виберіть варіант «`SaaS` app»).

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Loco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

Перейдіть до каталогу `myapp` за допомогою `cd` і запустіть застосунок:
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo loco start

                      ▄     ▀
                                ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
██████  █████   ███ █████   ███ █████   ███ ▀█
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
██████  █████   ███ █████       █████   ███ ████▄
██████  █████   ███ █████   ▄▄▄ █████   ███ █████
██████  █████   ███  ████   ███ █████   ███ ████▀
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

listening on port 5150
```
<!-- </snip> -->

## Працюють на Loco

* [SpectralOps](https://spectralops.io) — різноманітні сервіси на основі
  фреймворку Loco
* [Nativish](https://nativi.sh) — серверна частина застосунку на основі фреймворку Loco

## Учасники проєкту ✨

Дякуємо цим чудовим людям:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
