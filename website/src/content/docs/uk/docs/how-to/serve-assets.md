---
title: Обслуговування статичних та SPA-ресурсів
description: Обслуговуйте статичну теку (або односторінковий застосунок) за допомогою вбудованого middleware static, а потім за бажанням вбудуйте все в бінарник із функцією embedded_assets.
sidebar:
  order: 16
---

**Мета:** обслуговувати файли (зображення, CSS, JS, зібраний SPA-бандл) безпосередньо з Loco — з диска або вбудованими в скомпільований бінарник.

Це передбачає наявність робочого застосунку. Повну таблицю параметрів дивіться в записі `static` у [каталозі middleware](/uk/docs/reference/middleware/).

## 1. Покладіть файли під `assets/static/`

```
assets/
├── static/
│   ├── image.png
│   └── 404.html
└── views/
```

`assets/` розташована в корені проєкту, поруч із `src/` та `config/`.

## 2. Увімкніть middleware `static`

```yaml
# config/development.yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/static"
        path: "assets/static"
      fallback: "assets/static/404.html"
```

| Ключ | За замовчуванням | Призначення |
|---|---|---|
| `must_exist` | `true` | якщо `true`, відсутня налаштована тека є помилкою під час завантаження |
| `folder.uri` | `/static` | URL-префікс, за яким клієнти надсилають запити |
| `folder.path` | `assets/static` | тека на диску, що обслуговується |
| `fallback` | `assets/static/404.html` | файл, який подається, коли запитаний шлях не існує |
| `precompressed` | `false` | подавати сусідній файл `.gz` замість стиснення на льоту, якщо він існує |
| `cache_control` | `None` | наприклад, `"max-age=31536000, public"`; встановіть `null`, щоб повністю вимкнути заголовки кешування |

Посилайтеся на файли, що обслуговуються, з HTML/шаблонів як зазвичай:

```html
<img src="/static/image.png" />
```

## 3. Вимкніть fallback екрана вітання, якщо він затіняє ваші ресурси

Поза `Production` Loco за замовчуванням вмикає окреме middleware `fallback` («екран вітання Loco» для маршрутів без збігу), і воно має пріоритет над `static`. Якщо ваші статичні ресурси не з'являються, як очікувалося, у development, вимкніть його:

```yaml
server:
  middlewares:
    fallback:
      enable: false
```

## 4. Обслуговуйте односторінковий застосунок (SPA)

Направте `fallback` (у `static_assets`, а не в middleware `fallback` фреймворку з п. 3 вище) на `index.html` вашого SPA, щоб клієнтські маршрути коректно розв'язувалися під час жорсткого оновлення сторінки:

```yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "assets/static"
      fallback: "assets/static/index.html"
```

Будь-який запит, що не збігається з реальним файлом під `assets/static/`, падає у `index.html`, дозволяючи вашому клієнтському роутеру взяти керування.

Якщо SPA — це власний clientside-режим Loco (фронтенд Vite/React у `frontend/` з TypeScript-типами, згенерованими з ваших Rust DTO), ця конфігурація вже згенерована за вас (вказує на `frontend/dist`). Дивіться [Створюємо типізований React SPA](/uk/docs/how-to/build-a-spa/).

## 5. Обслуговуйте попередньо стиснуті ресурси

Якщо ваш пайплайн збірки вже створює файли `.gz` поруч з оригіналами (наприклад, `app.js` і `app.js.gz`), увімкніть `precompressed`, і Loco подаватиме варіант `.gz` безпосередньо замість стиснення на кожен запит:

```yaml
server:
  middlewares:
    static:
      enable: true
      precompressed: true
```

## 6. Вбудуйте ресурси в бінарник із `embedded_assets`

Для розгортання одним бінарником (без окремої теки ресурсів для доставки чи монтування) увімкніть Cargo-функцію `embedded_assets`:

```toml
[dependencies]
loco-rs = { version = "...", features = ["embedded_assets"] }
```

Це **заміна на етапі компіляції**, а не окремий API — той самий ключ конфігурації `server.middlewares.static` і ті самі параметри застосовуються, і ваші контролери/шаблони взагалі не змінюються:

- реалізація middleware `static` змінюється з читання `assets/static/` з диска на подання файлів, вбудованих у бінарник під час збірки
- рушій шаблонів Tera (`TeraView`) так само змінюється на вбудований варіант, який подає скомпільовані в бінарник шаблони замість читання `assets/views/` з диска

Під час збірки ви побачите вивід логу, що підтверджує, що саме було вбудовано:

```
warning: loco-rs@x.y.z: Discovered directories for assets:
warning: loco-rs@x.y.z:   - /path/to/app/assets/static
warning: loco-rs@x.y.z:   - /path/to/app/assets/views
warning: loco-rs@x.y.z: Found asset: /path/to/app/assets/static/image.png -> /static/image.png
warning: loco-rs@x.y.z: Found 6 asset files
warning: loco-rs@x.y.z: Generated code for 6 static assets and 7 templates
```

Компроміси: бінарник зростає приблизно на розмір `assets/`, і будь-яка зміна ресурсу вимагає повної перекомпіляції — циклу «відредагував і оновив», як при поданні з диска, немає. Увімкнюйте/вимикайте цю функцію для різних профілів збірки (наприклад, вбудовані ресурси для release, файлова система для локальної розробки), якщо ця вартість перекомпіляції заважає під час активної роботи з ресурсами.

## Перевірка

```sh
curl -I localhost:5150/static/image.png    # 200, правильний Content-Type
curl -I localhost:5150/static/does-not-exist.png   # падає у fallback відповідно до конфігурації
```

## Далі

- [Рендеринг серверних шаблонів](/uk/docs/how-to/render-views/)
- [Додаємо middleware](/uk/docs/how-to/add-middleware/)
- [Каталог middleware](/uk/docs/reference/middleware/)
