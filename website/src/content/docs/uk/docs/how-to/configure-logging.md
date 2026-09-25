---
title: Налаштування логування
description: Встановіть рівень і формат логера, зрозумійте пріоритет фільтрації та додайте ротаційний file appender.
sidebar:
  order: 33
---

Мета: контролювати, що логує Loco, у якій формі та куди — stdout для розробки, структурований JSON для production і (опційно) ротаційний файл логів — без потопання в шумі сторонніх крейтів.

Логер Loco побудований на `tracing`. Ключі `logger.enable`, `logger.level` і `logger.format` є обов'язковими в кожному конфігураційному файлі.

## 1. Встановіть мінімальну конфігурацію

```yaml
# config/development.yaml
logger:
  enable: true
  pretty_backtrace: true
  level: debug
  format: compact
```

- `level`: `off` | `trace` | `debug` | `info` | `warn` | `error`.
- `format`: `compact` | `pretty` | `json`.
- `pretty_backtrace`: коли `true`, примусово вмикає `RUST_BACKTRACE=1` і гарно відформатовані backtrace панік. Це зручність для розробки — вимикайте її в продуктивних розгортаннях, чутливих до швидкодії.

## 2. Знайте пріоритет фільтрації

Loco не просто застосовує `level` глобально до кожного крейту — за замовчуванням він вносить у білий список невеликий набір модулів (`loco_rs`, `sea_orm_migration`, `tower_http`, `sqlx::query`, `playground`, `loco_gen`) плюс крейт вашого власного застосунку, і застосовує `level` лише до них. Усе інше залишається тихим.

Три способи контролювати це, у строгому порядку пріоритету:

1. **Змінна середовища `RUST_LOG`** — якщо встановлена, вона перемагає беззастережно, ігноруючи і `level`, і `override_filter`. Використовуйте це для разового налагодження на запущеному процесі без дотику до конфігурації:
   ```sh
   RUST_LOG=debug cargo loco start
   ```
2. **`logger.override_filter`** — сирий рядок директиви `EnvFilter` з `tracing-subscriber`. Використовуйте це, щоб назавжди бачити трейси з бібліотек поза вбудованим білим списком:
   ```yaml
   logger:
     enable: true
     level: info
     format: compact
     override_filter: "trace" # або директива на кшталт "myapp=debug,tower_http=debug"
   ```
3. **Вбудований білий список модулів + `level`** — те, що ви отримаєте, якщо не встановлено жодного з перших двох. Це типовий випадок: встановіть `level` і довіряйте білому списку Loco, який тримає шум у межах.

## 3. Оберіть формат для кожного середовища

- `compact` — придатний для людини однорядковий вивід; добрий стандарт для локальної розробки.
- `pretty` — багаторядковий, більш просторий вивід для людини.
- `json` — структурований, один JSON-об'єкт на рядок; використовуйте його в production, щоб агрегатор логів (Loki, CloudWatch, Datadog тощо) міг розбирати поля напряму.

```yaml
# config/production.yaml
logger:
  enable: true
  pretty_backtrace: false
  level: info
  format: json
```

## 4. Додайте ротаційний file appender

`file_appender` пише логи на диск незалежно від логера stdout (та з власними level/format, окремими від нього) — корисно, коли ви хочете тримати stdout тихим, але все одно захоплювати повний слід на диску.

```yaml
logger:
  enable: true
  level: info
  format: compact
  file_appender:
    enable: true
    non_blocking: false   # true переносить запис у фоновий потік
    level: debug
    format: json
    rotation: daily       # minutely | hourly | daily | never — стандартно hourly
    dir: ./logs           # стандартно "./logs", якщо пропущено
    filename_prefix: myapp
    filename_suffix: log
    max_log_files: 7      # обов'язково — старі файли понад цю кількість видаляються
```

З `rotation: daily` і наведеними вище налаштуваннями ви отримаєте файли на кшталт `./logs/myapp.<дата>.log`, що ротуються раз на день, зі збереженням лише 7 найновіших.

## 5. Перевірте

Запустіть застосунок і переконайтеся, що з'являється очікуваний формат:

```sh
cargo loco start
# ... спостерігайте за stdout: рядки формату compact/pretty/json на встановленому рівні
```

Якщо ви увімкнули file appender, спостерігайте за текою логів:

```sh
tail -f ./logs/*.log
```

Щоб переконатися в пріоритеті фільтрації, спробуйте перевизначити під час виконання, не чіпаючи конфігураційний файл:

```sh
RUST_LOG=loco_rs=trace cargo loco start
```

Ви маєте побачити значно детальніший вивід, ніж дало б лише `logger.level` — це підтверджує, що `RUST_LOG` має пріоритет.

## Довідник

- Кожен YAML-ключ `logger:`, включно з усіма під-ключами `file_appender`: [довідник конфігурації § logger](/uk/docs/reference/configuration/#logger)
