---
title: Додаємо websockets / realtime
description: У Loco немає вбудованого websocket-шару — підключіть realtime за допомогою зовнішнього Axum-сумісного крейту, як-от socketioxide.
sidebar:
  order: 17
---

Мета: додати до Loco-застосунку реальний двонаправлений зв'язок (чат, живі оновлення, сповіщення).

Loco не постачає вбудованої websocket-абстракції. Оскільки Loco-застосунок компілюється у справжній `axum::Router<AppContext>` (дивіться [Прихід з Axum](/uk/docs/explanation/coming-from-axum/), будь-який Axum-сумісний websocket-шар монтується на нього так само, як і на застосунок на чистому Axum — тут немає специфічного для Loco API, який треба вивчати, і немає специфічних для Loco обмежень.

## Приклад кімнати чату

Робочий приклад з використанням [socketioxide](https://github.com/Totodore/socketioxide) дивіться в референсному застосунку [`loco-rs/chat-rooms`](https://github.com/loco-rs/chat-rooms). Він показує повну реалізацію чат-кімнати, підключену до роутера Loco.

Якщо вам потрібне щось інше, ніж socketioxide, шукайте будь-який крейт, що інтегрується з `axum::Router` (raw `axum::extract::ws`, `socketioxide` тощо) і монтуйте його так само, як ви [додаєте контролер](/uk/docs/how-to/add-controller/) — як маршрути на `Router` застосунку.
