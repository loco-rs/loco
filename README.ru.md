 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Добро пожаловать в *Loco*</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · Русский · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · [العربية](./README.ar.md) · [Bahasa Indonesia](./README.id.md)


## Что такое Loco?
*Loco* сильно вдохновлён проектом *Ruby on Rails*. Если вы знакомы и с *Rails*, и с *Rust*, вы будете чувствовать себя как дома. Если вы знаете только *Rails*, и не знакомы с *Rust*, *Loco* будет для вас чем-то освежающим.

Если вам интересно узнать внутрение устройство *Loco*, включая детальные гайды, примеры, и устройство API, почитайте нашу [документацию](https://loco.rs).


## Фишки Loco:

- **Простота превыше конфигурации**: Подобно *Ruby on Rails*, *Loco* делает упор на простоту и продуктивность, снижая потребность в лишнем коде. *Loco* использует оптимальные настройки по-умолчанию, давая разработчикам возможность сфокусироваться на написании бизнес логики, а не конфигурации.
- **Быстрая разработка**: Ставя акцент на высокой производительности разработчика, Дизайн *Loco* фокусируется на сокращении ненужного кода и предоставления интуитивного API. Это позволяет быстро создавать прототипы без лишних усилий.
- **ORM интеграция**: Стройте свой бизнес с крепкими составляющими, убирая необходимость писать SQL. Определяйте взаимосвязи, проверку, и кастомную логику прямо в составляющих, упрощая поддержку и рост кодовой базы.
- **Контролеры**: Обрабатывайте параметры и данные web-запросов, проверяйте их содержимое, отображайте ответ с учетом запроса. Мы используем *Axum* для достижения наилучшей производительности, простоты, и возможности расширения. Также, контролеры облегчают внедрение middleware. Это может быть использовано для добавления всевозможной логики: аутентификации, логгинга, или обработки ошибок перед отправкой на сервер.
- **Виды**: *Loco* может интегрироваться с template-движками для генерации динамического HTML из шаблонов.
- **Фоновые задачи**: Исполняйте I/O и другие тяжелые операции в фоновом режиме с помощью *Redis*, или потоков. Для написания функционала фоновой задачи нужно всего лишь написать функцию `perform` из `trait Worker`.
- **Планировщик**: Облегчает традиционную, часто громоздкую систему, упрощая планировку задач и исполнение shell-скриптов.
- **Отправка электронной почты**: Отправка электронной почты в фоновом режиме, без необходимости создавать новую фоновую задачу.
- **Хранилище**: Мы способствуем работе с файлами несколькими путями: хранение в памяти, на диске, или использование облачных сервисов как *AWS*, *S3*, *GCP*, и *Azure*.
- **Кэширование**: *Loco* кэширует частые запросы для улучшения производительности приложения.

У *Loco* есть ещё множество фишек, котрые вы можете посмотреть на [сайте документации](https://loco.rs/docs/getting-started/tour/).


## Установка
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Теперь вы можете создать свое новое приложение (выберете "`SaaS` app").


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

Теперь выполните `cd` в папку `myapp` и запускайте приложение:
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

## Проекты, использующие *Loco*
+ [SpectralOps](https://spectralops.io) - различные сервисы, использующие *Loco*
  framework
+ [Nativish](https://nativi.sh) - backend приложения, использующий *Loco*

## Контрибьютеры ✨
Спасибо всем этим прекрасным людям:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
