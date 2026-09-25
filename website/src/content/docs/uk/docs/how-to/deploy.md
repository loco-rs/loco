---
title: Розгортання в production
description: Зберіть release-бінарник, згенеруйте Dockerfile або nginx-конфіг за допомогою cargo loco generate deployment та перевірте production-конфігурацію перед випуском.
sidebar:
  order: 32
---

Мета: запустити застосунок Loco на продакшн-хості. Loco компілюється в єдиний самодостатній бінарник — цільовому серверу не потрібні ані `cargo`, ані Rust-тулчейн, лише бінарник і тека `config/`.

## 1. Зберіть release-бінарник

```sh
cargo build --release
```

Назва вашого бінарника відповідає `[package] name` у `Cargo.toml` (з суфіксом `-cli`, напр. `myapp-cli`) і потрапляє до `./target/release/`.

## 2. Згенеруйте Dockerfile (опційно)

```sh
cargo loco generate deployment docker
```

`kind` — **позиційний** аргумент — `docker`, `nginx` або `lambda`, а не прапорець `--kind`.

Це записує два файли до кореня вашого проєкту:

- `Dockerfile` — багатостадійна збірка: компіляція через `cargo build --release` у builder-стадії `rust:slim`, потім копіювання лише скомпільованого бінарника та `config/` у slim runtime-образ `debian:bookworm-slim`. Якщо ваш застосунок має `frontend/package.json` (клієнтський рендеринг), у builder-стадії також встановлюється Node і виконується `npm install && npm run build`. Якщо налаштовано `server.middlewares.static_assets`, теки, на які він вказує, теж копіюються до фінального образу.
- `.dockerignore` — виключає `target/`, `.git` та інші артефакти збірки з контексту Docker-збірки.

Збирайте та запускайте його, як будь-який інший образ:

```sh
docker build -t myapp .
docker run -p 5150:5150 --env-file .env myapp
```

## 3. Згенеруйте nginx-конфіг (опційно)

```sh
cargo loco generate deployment nginx
```

Це записує `nginx/default.conf` — конфіг reverse-proxy, похідний від ваших поточних `server.host` / `server.port` (`config/<env>.yaml`), — який проксує як голий домен, так і wildcard-піддомени до вашого застосунку.

## 4. Розгортання на AWS Lambda (опційно)

```sh
cargo loco generate deployment lambda
```

Loco будує стандартний `Router` Axum, а і Axum, і runtime AWS Lambda є `tower::Service` — тож ваш застосунок працює на Lambda без переписування. Це записує:

- `src/bin/lambda.rs` — точку входу Lambda, яка запускає ваш застосунок у режимі `ServerOnly` і передає роутер HTTP-runtime Lambda (`lambda_http::run`). Це окремий binary target, тож `cargo loco start` і ваш CLI залишаються недоторканими.
- додає `lambda_http` до вашого `Cargo.toml`.
- записує блок `[package.metadata.lambda]` до вашого `Cargo.toml`, щоб збірка/розгортання потребували **жодних додаткових прапорців** — він оголошує, які runtime-файли потрапляють у zip (`config/`, плюс `assets/` тощо, якщо виявлено) і розумні стандартні значення розгортання (`memory`, `timeout`, `LOCO_ENV=production`). Нічого специфічного для середовища не вшито — регіон, акаунт, IAM-роль і секрети надаються під час розгортання.

Потім розгортайте за допомогою [cargo-lambda](https://www.cargo-lambda.info) — дві команди, без прапорців:

```sh
cargo install cargo-lambda
cargo lambda build --release --arm64 --output-format zip
cargo lambda deploy --enable-function-url
```

`deploy` створює функцію, роль виконання та публічну **Function URL**, а потім виводить HTTPS-ендпоінт. Встановіть секрети через `cargo lambda deploy --enable-function-url --env-var DATABASE_URL=... --env-var JWT_SECRET=...` (або підключіть Secrets Manager).

### Кінцевий результат

`cargo lambda build` створює `.zip` у `target/lambda/lambda/`, що містить єдиний виконуваний файл `bootstrap` (ваш скомпільований Rust-бінарник) плюс runtime-файли, оголошені в metadata `include`. Цей zip — *весь* артефакт, який надсилається в AWS — керованого runtime-шару немає; він працює на кастомному runtime `provided.al2023`. **Виміряно для стандартного застосунку з БД:** ~18 МБ розпаковано → **~8 МБ zip** — значно нижче ліміту прямого завантаження Lambda (50 МБ zip / 250 МБ розпаковано), тож S3-стейджинг не потрібен. Надавайте перевагу `--arm64` (Graviton) заради нижчої вартості та швидших холодних стартів.

**Що постачається понад бінарник:** усе, що Loco читає з диска під час виконання — завжди `config/`, плюс `assets/`, файли i18n і шаблони з `src/mailers/`, якщо ваш застосунок обслуговує перегляди/статичні ресурси/i18n/пошту. Генератор виявляє їх і перераховує в metadata `include`; додайте туди ще записи, якщо ви читаєте інші файли під час виконання. (Альтернативно — контейнеризуйте, дивіться нижче, що пакує всю теку застосунку.)

### Що це торкається на боці AWS

Понад саму функцію, робоче розгортання передбачає:

- **Передню двері.** [Lambda Function URL](https://docs.aws.amazon.com/lambda/latest/dg/urls-configuration.html) або API Gateway (v2 HTTP API). `lambda_http` прозоро обробляє всі три форми подій (Function URL, API GW v1/v2).
- **IAM-роль виконання.** Як мінімум права на CloudWatch Logs (`cargo lambda deploy` може створити базову роль, або передайте `--role`); додайте права доступу до VPC, якщо прикріплюєтеся до VPC, плюс права для всього, що викликає застосунок (S3, SES, Secrets Manager).
- **Логи.** Вивід tracing Loco йде в stdout → CloudWatch Logs. Використовуйте JSON-логування у production.
- **Конфігурація та секрети.** Встановіть `LOCO_ENV` і секрети (`DATABASE_URL`, JWT-секрет, ...) як змінні середовища функції (`--env-var`) або через Secrets Manager/SSM.
- **Мережа до бази даних.** Щоб дістатися RDS у VPC, прикріпіть функцію до підмерет VPC + security group (потрібна роль доступу до VPC). Вихідні виклики до SES/S3/Secrets Manager тоді потребують NAT-шлюзу або VPC-ендпоінтів. **Серйозно розгляньте [RDS Proxy](https://docs.aws.amazon.com/lambda/latest/dg/services-rds-tutorial.html):** кожен теплий інстанс Lambda тримає власні з'єднання з БД, тож масштабування може вичерпати ліміти з'єднань Postgres — RDS Proxy пулить їх.

### Примітки та обмеження

- **Лише HTTP.** Фонові воркери та планувальник не запускаються в точці входу Lambda — вони не вписуються в модель request/response Lambda. Запускайте їх на завжди-увімкненій цілі (ECS/EC2) або керуйте ними через SQS/EventBridge.
- **Тримайте міграції поза шляхом запитів.** Виконуйте `cargo loco db migrate` з CI або разової задачі, і направте runtime `LOCO_ENV` на конфігурацію, яка не мігрує автоматично під час завантаження.
- **Холодні старти.** Rust стартує швидко, але Loco завантажує весь застосунок (включно зі з'єднанням з БД) на кожен холодний старт; прикріплення до VPC додає затримку налаштування ENI. Зарезервована конкурентність (provisioned concurrency) згладжує це за потреби.
- **Хочете нуль змін коду?** Ви можете натомість контейнеризувати свій звичайний бінарник із [AWS Lambda Web Adapter](https://github.com/awslabs/aws-lambda-web-adapter) поверх згенерованого `Dockerfile` — `lambda.rs` не потрібен, і вся тека застосунку (config, assets) потрапляє в образ.

## 5. Встановіть змінні середовища, яких вимагає production

Окремого «production-режиму» немає — Loco обирає конфігураційний файл за середовищем, і **середовище за замовчуванням — `development`**. Встановіть `LOCO_ENV=production` на сервері (або передайте `--environment production`), інакше ваш застосунок там читатиме `config/development.yaml` і виглядатиме робочим.

`config/production.yaml` було написано за вас під час `loco new`, уже налаштованим для production: backtrace вимкнено, логи `json`, прив'язка на `0.0.0.0`, справжній пул з'єднань.

Чого він навмисно **не** містить, так це жодного секрету чи адреси. Вони читаються з середовища без резервних значень:

| Змінна | Використовується для | Обов'язкова |
| --- | --- | --- |
| `DATABASE_URL` | З'єднання з базою даних | так, якщо є база даних |
| `JWT_SECRET` | Підписання та перевірка токенів | так, якщо є auth |
| `HOST` | Публічний URL, на який вказують посилання у вихідних листах | так |
| `MAILER_HOST`, `MAILER_USER`, `MAILER_PASSWORD` | SMTP-сервер і облікові дані | так, якщо є мейлер |
| `REDIS_URL` / `QUEUE_URL` | Бекенд черги | так, якщо є черга |
| `PORT`, `BINDING`, `LOG_LEVEL`, `DB_MAX_CONNECTIONS`, `DB_AUTO_MIGRATE` | Перевизначення | ні, є розумні стандартні значення |

Відсутня обов'язкова змінна зупиняє застосунок під час старту з назвою цієї змінної, замість того щоб дозволити йому працювати з розробницьким секретом або вказувати на базу даних, якої немає. Це навмисно: збій завантаження, який можна прочитати, кращий за застосунок, що виглядає здоровим і підписує токени ключем, закоміченим до вашого репозиторію.

```sh
export DATABASE_URL='postgres://user:password@db-host:5432/myapp_production'
export JWT_SECRET="$(openssl rand -hex 32)"
export HOST='https://myapp.example.com'
```

Якщо ви запускаєте понад один інстанс або виконуєте міграції як окремий крок релізу, встановіть `DB_AUTO_MIGRATE=false`, щоб інстанси не змагалися за міграцію однієї бази даних.

Дивіться [Налаштування логування](/uk/docs/how-to/configure-logging/) щодо логів і [довідник конфігурації](/uk/docs/reference/configuration/) щодо кожного ключа у файлі.

## 6. Запустіть `loco doctor` перед виходом у продакшн

```sh
myapp-cli doctor --environment production
```

Виконайте це на сервері, де знаходяться змінні середовища та база даних, яку застосунок фактично використовуватиме. `doctor` відкриває production-конфігурацію, з'єднується з названими БД і чергою, а додатково повідомляє про налаштування, які безпечні в розробці, але не в production — прив'язку до loopback, `dangerously_truncate`, увімкнені backtrace.

Додайте `-c`/`--config`, щоб вивести повністю розв'язану конфігурацію, після підстановки змінних середовища, для перевірки:

```sh
myapp-cli doctor --config --environment production
```

`--production` досі працює як застарілий аліас для `--environment production`.

## 7. Доставте це

Скопіюйте бінарник і теку `config/` на сервер (не потрібні ні вихідні коди, ні `Cargo.lock`, ні тулчейн):

```sh
scp target/release/myapp-cli config/ user@server:/opt/myapp/
ssh user@server 'LOCO_ENV=production /opt/myapp/myapp-cli start'
```

## Перевірте

- `myapp-cli doctor --environment production` завершується з кодом 0 і повідомляє, що всі перевірки пройдено.
- `myapp-cli start` завантажується, і стартовий банер показує очікувані середовище, БД і логер.
- Звернення до health/root-маршруту застосунку через nginx (якщо ви його згенерували) повертає відповідь, що підтверджує правильність підключення reverse proxy до хоста/порту.

## Довідник

- Форма CLI `generate deployment` (`docker`/`nginx`/`lambda` як `kind`): [довідник CLI](/uk/docs/reference/cli/)
- Кожен згаданий вище конфігураційний ключ (`logger`, `server`, `database`, `auth`, `mailer`, `queue`): [довідник конфігурації](/uk/docs/reference/configuration/)
