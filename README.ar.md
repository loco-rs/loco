<div dir="rtl" align="center">

  <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

  <h1>مرحبًا بك في Loco</h1>

  <h3>
🚂 Loco هو Rust on Rails.
  </h3>

  [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
  [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
  [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

</div>

<div dir="rtl">

[English](./README.md) · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md) · [Vietnamese](./README.vi.md) · العربية · [Bahasa Indonesia](./README.id.md)

---

## ما هو Loco؟

&#x200F;`Loco` مستوحى بشكل كبير من `Rails`. إذا كنت تعرف `Rails` و `Rust`، فستشعر وكأنك في بيتك. أما إذا كنت تعرف `Rails` فقط وحديث العهد بـ `Rust`، فستجد `Loco` منعشًا. نحن لا نفترض معرفتك المسبقة بـ `Rails`.

للتعمق أكثر في طريقة عمل `Loco`، بما في ذلك الأدلة المفصلة والأمثلة ومراجع `API`، طالع [موقعنا التوثيقي](https://loco.rs).

## مميزات Loco:

* &#x200F;`Convention Over Configuration:` مثل `Ruby on Rails`، تركز `Loco` على البساطة والإنتاجية بتقليل الحاجة إلى كتابة `code` مكرر. تستخدم إعدادات افتراضية منطقية، مما يتيح للمطورين التركيز على `business logic` بدلًا من قضاء الوقت في الإعدادات.
* &#x200F;`Rapid Development:` تهدف `Loco` إلى إنتاجية عالية للمطورين من خلال تقليل `boilerplate code` وتوفير `APIs` بديهية، مما يتيح التطوير السريع وبناء `prototypes` بأقل جهد.
* &#x200F;`ORM Integration`: نمذجة منطق عملك باستخدام `entities` قوية، مما يلغي الحاجة إلى كتابة `SQL`. حدد العلاقات والتحقق (`validation`) والمنطق المخصص مباشرة على `entities` لتحسين قابلية الصيانة والتوسعة.
* &#x200F;`Controllers`: تتولى `Controllers` معالجة طلبات الويب من حيث `parameters` و `body` و `validation`، وعرض `response` متوافقة مع المحتوى. نستخدم `Axum` لأفضل أداء وبساطة وقابلية توسع. تتيح لك `Controllers` أيضًا بناء `middlewares` بسهولة لإضافة `Authentication` والتسجيل ومعالجة الأخطاء قبل تمرير الطلبات إلى `main controller actions`.
* &#x200F;`Views`: يمكن لـ `Loco` التكامل مع `templating engines` لإنشاء محتوى `HTML` ديناميكي من `templates`.
* &#x200F;`Background Jobs`: تنفيذ المهام المُكثفة (`compute` أو `I/O`) في الخلفية باستخدام `Redis-backed queue` أو `threads`. إنشاء `worker` بسيط مثل تنفيذ دالة `perform` لـ `Worker` trait.
* &#x200F;`Scheduler`: يُبسّط نظام `crontab` التقليدي المُعقّد، مما يجعله أكثر أناقة وسهولة لجدولة المهام أو `shell scripts`.
* &#x200F;`Mailers`: يقوم `mailer` بتسليم البريد الإلكتروني في الخلفية باستخدام البنية التحتية لـ `background worker` في `Loco`، لتكون العملية سلسة تمامًا.
* &#x200F;`Storage`: في `Loco Storage`، نُسهّل التعامل مع الملفات بطرق متعددة: `in-memory`، على القرص، أو من خلال خدمات سحابية مثل `AWS S3` و `GCP` و `Azure`.
* &#x200F;`Cache`: توفر `Loco` طبقة `cache` لتحسين أداء التطبيق عن طريق تخزين البيانات كثيرة الاستخدام.

لمزيد من الميزات، طالع [موقعنا التوثيقي](https://loco.rs/docs/getting-started/tour/).

## البدء

```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```

الآن يمكنك إنشاء تطبيقك الجديد (اختر "`SaaS` app"):

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

الآن `cd` إلى `myapp` وشغّل تطبيقك:

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

## مدعوم من Loco

+ [SpectralOps](https://spectralops.io) - خدمات متنوعة مدعومة بإطار `Loco`
+ [Nativish](https://nativi.sh) - تطبيق `backend` مدعوم بإطار `Loco`

## المساهمون ✨

الشكر لهؤلاء المساهمين الرائعين:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>

</div>
