---
title: Планування повторюваних завдань
description: Налаштуйте планувальник, щоб виконувати таск або shell-команду за cron-розкладом чи розкладом англійською мовою.
sidebar:
  order: 22
---

Мета: виконувати [таск](/uk/docs/how-to/write-task/) або shell-команду за повторюваним розкладом, не вигадуючи власний `crontab`.

## 1. Створюємо конфігурацію планувальника

Згенеруйте окремий файл:

```sh
cargo loco generate scheduler
```

Це створить `config/scheduler.yaml`. Альтернативно додайте блок `scheduler:` безпосередньо до YAML-файлу вашого середовища (`config/development.yaml` тощо) — обидві форми використовують одну схему.

## 2. Описуємо завдання

```yaml
scheduler:
  output: stdout # типовий вивід для всіх завдань: stdout | silent
  jobs:
    write_content:
      shell: true # виконувати `run` як shell-команду (за замовчуванням: false = виконати таск)
      run: "echo loco >> ./scheduler.txt"
      schedule: run every 1 second # синтаксис англійською
      output: silent # перекриває типовий рівень для цього завдання
      tags: ["base", "infra"]

    run_task:
      run: "foo" # назва зареєстрованого таска
      schedule: "at 10:00 am"
      run_on_start: true # виконати ще раз одразу при старті планувальника

    list_if_users:
      run: "user_report"
      shell: true
      schedule: "* 2 * * * *" # синтаксис cron
      tags: ["base", "users"]
```

Кожен запис завдання має:

| Ключ | Обов'язковий? | Примітки |
|---|---|---|
| `run` | так | Shell-команда (якщо `shell: true`) або назва зареєстрованого таска з необов'язковими аргументами `KEY:VALUE` (якщо `shell: false`, за замовчуванням) |
| `schedule` | так | Фраза англійською або вираз cron — дивіться нижче |
| `shell` | ні, за замовчуванням `false` | `false` виконує `run` як таск; `true` — як shell-команду |
| `run_on_start` | ні, за замовчуванням `false` | Також спрацювати один раз одразу при старті планувальника |
| `tags` | ні | Групує завдання, щоб запускати їх разом через `--tag` |
| `output` | ні | Перекриває `scheduler.output` лише для цього завдання |

### Синтаксис розкладу

`schedule` приймає будь-яку з двох форм — Loco автоматично розпізнає синтаксис cron, перевіряючи, чи починається рядок з цифри або `*`; усе інше розбирається як англійська фраза через `english_to_cron`:

- Англійською: `every 15 seconds`, `run every minute`, `fire every day at 4:00 pm`, `at 10:00 am`, `run at midnight on the 1st and 15th of the month`, `On Sunday at 12:00`, `7pm every Thursday`, `midnight on Tuesdays`
- Cron (7 полів, **UTC**, включно з секундами та роком):

  ```
  sec   min   hour   day of month   month   day of week   year
  *     *     *      *              *       *             *
  ```

## 3. Перевіряємо конфігурацію

```sh
# окремий файл
cargo loco scheduler --config config/scheduler.yaml --list

# блок scheduler:, вбудований у файл середовища
LOCO_ENV=production cargo loco scheduler --list
```

## 4. Запускаємо

Як окремий процес:

```sh
cargo loco scheduler                                    # використовує scheduler: з config/<env>.yaml
cargo loco scheduler --config config/scheduler.yaml      # використовує окремий файл
```

Або в одному процесі разом із сервером і воркером:

```sh
cargo loco start --all
```

Якщо ваші завдання живуть в окремому `scheduler.yaml`, а не вбудовані у файл середовища, `start --all` треба повідомити, де його шукати — задайте `SCHEDULER_CONFIG`:

```sh
SCHEDULER_CONFIG=config/scheduler.yaml cargo loco start --all
```

Кожен запуск створює **субпроцес** (`/bin/sh -c` на Unix, `cmd.exe /C` на Windows); `LOCO_ENV` передається йому, тож завдання-таск розв'язує ту саму конфігурацію/середовище, що й батьківський процес. При зупинці (Ctrl+C) планувальник чекає на завдання, що виконуються, перед виходом.

## 5. Запускаємо підмножину за назвою або тегом

```sh
LOCO_ENV=production cargo loco scheduler --name 'run_task'
LOCO_ENV=production cargo loco scheduler --tag 'base'
```

## Довідка

- Написання таска, який викликає завдання планувальника: [Пишемо разовий таск](/uk/docs/how-to/write-task/)
- Ключі конфігурації `scheduler`/`SCHEDULER_CONFIG`: [Довідник конфігурації](/uk/docs/reference/configuration/)
- Прапорці `cargo loco scheduler`: [Довідник CLI](/uk/docs/reference/cli/)
