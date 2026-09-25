// Locale detection + string lookup for the marketing pages (home, blog,
// casts, privacy policy) — the part of the site that is NOT Starlight.
//
// Convention: a marketing page lives at `/uk/<path>/` when it is Ukrainian
// and `<path>/` at the root when it is English. Astro localizes Starlight
// routes via `Astro.currentLocale`, but plain marketing pages don't get that
// for free, so each page derives its locale from its own URL and passes it
// to the shared components.

export type Locale = 'en' | 'uk';

export const locales: Locale[] = ['en', 'uk'];

/** Human labels used in the language switcher. */
export const localeLabels: Record<Locale, string> = {
  en: 'English',
  uk: 'Українська',
};

const dictionaries: Record<Locale, Record<string, string>> = {
  en: {
    'nav.docs': 'Docs',
    'nav.blog': 'Blog',
    'nav.casts': 'Screencasts',
    'nav.github': '★ GitHub',
    'nav.getStarted': 'Get started',

    'hero.eyebrow': '● Batteries-included Rust',
    'hero.title1': 'The',
    'hero.title2': 'one-person',
    'hero.title3': 'framework for Rust.',
    'hero.lede':
      'Everything you need to take a side-project or startup from an idea to production — models, controllers, jobs, mailers, auth — generated and wired together.',
    'hero.cta': 'Start building →',
    'hero.copy': '⧉ copy',
    'hero.trust.version': 'Sea-ORM 2.0 · edition 2024',
    'hero.trust.stars': 'Stars',
    'hero.trust.starsOn': 'on GitHub',
    'hero.trust.prod': '0 → prod',
    'hero.trust.prodSuffix': 'in one command',

    'strip.1': 'batteries-included',
    'strip.2': 'Rails, in Rust',
    'strip.3': 'SeaORM',
    'strip.4': 'Axum',
    'strip.5': 'background jobs',
    'strip.6': 'test-driven',

    'pillars.head': 'Empower the one-person team.',
    'pillars.sub':
      'Loco follows Rails — carefully adapted to modern Rust. The heavy lifting is tucked away so you can move fast, and pulled back out the moment you need to scale.',
    'pillars.1.title': 'Batteries included',
    'pillars.1.copy':
      'Service, data, emails, background jobs, tasks, and a CLI to drive it all — in the box from day one.',
    'pillars.2.title': 'Rails is great',
    'pillars.2.copy':
      'Loco follows Rails. There, we said it — its concepts, carefully adapted to idiomatic Rust.',
    'pillars.3.title': 'Deliver with confidence',
    'pillars.3.copy':
      'Unapologetically optimized for the solo developer. Complexity and heavy lifting tucked away.',
    'pillars.4.title': 'Scale when needed',
    'pillars.4.copy':
      'Split, reconfigure, or use only the parts of Loco you need. Grow without a rewrite.',
    'pillars.5.title': 'Build incrementally',
    'pillars.5.copy':
      'Just a service. Or with a database. Or a background worker. Or a task. Use what you need.',
    'pillars.6.title': 'Test-driven everything',
    'pillars.6.copy':
      'Models, controllers, jobs — test the whole app with very little effort. Ship fast, stay safe.',

    'deck.head': 'One feature, a few small files.',
    'deck.prev': 'Previous part',
    'deck.next': 'Next part',

    'footer.tagline': '© Loco — the one-person framework for Rust',
    'footer.docs': 'Docs',
    'footer.github': 'GitHub',
    'footer.discord': 'Discord',
    'footer.blog': 'Blog',

    'home.title': 'Loco — the one-person framework for Rust',
    'home.description': 'Loco is the one-person framework for Rust.',

    'privacy.title': 'Privacy Policy',
    'privacy.description': 'We do not use cookies and we do not collect any personal data.',
    'privacy.tldr': 'TLDR',
    'privacy.tldrText': 'We do not use cookies and we do not collect any personal data.',
    'privacy.visitors': 'Website visitors',
    'privacy.visitors1': 'No personal information is collected.',
    'privacy.visitors2': 'No information is stored in the browser.',
    'privacy.visitors3': 'No information is shared with, sent to or sold to third-parties.',
    'privacy.visitors4': 'No information is shared with advertising companies.',
    'privacy.visitors5': 'No information is mined and harvested for personal and behavioral trends.',
    'privacy.visitors6': 'No information is monetized.',
    'privacy.contact': 'Contact us',
    'privacy.contactText': 'if you have any questions.',
    'privacy.effective': 'Effective Date: 1st May 2021',
  },
  uk: {
    'nav.docs': 'Документація',
    'nav.blog': 'Блог',
    'nav.casts': 'Скринкасти',
    'nav.github': '★ GitHub',
    'nav.getStarted': 'Почати',

    'hero.eyebrow': '● Rust «з батарейками в комплекті»',
    'hero.title1': 'Фреймворк',
    'hero.title2': 'для одного розробника',
    'hero.title3': 'на Rust.',
    'hero.lede':
      'Усе, що потрібно, щоб провести сайдпроєкт чи стартап від ідеї до продакшену — моделі, контролери, фонові завдання, пошта, автентифікація — згенеровані та зібрані докупи.',
    'hero.cta': 'Почати розробку →',
    'hero.copy': '⧉ копіювати',
    'hero.trust.version': 'Sea-ORM 2.0 · edition 2024',
    'hero.trust.stars': 'Зірок',
    'hero.trust.starsOn': 'на GitHub',
    'hero.trust.prod': '0 → прод',
    'hero.trust.prodSuffix': 'однією командою',

    'strip.1': 'батарейки в комплекті',
    'strip.2': 'Rails, тільки Rust',
    'strip.3': 'SeaORM',
    'strip.4': 'Axum',
    'strip.5': 'фонові завдання',
    'strip.6': 'тестування усього',

    'pillars.head': 'Розширте можливості команди з однієї людини.',
    'pillars.sub':
      'Loco наслідує Rails — дбайливо адаптований до сучасного Rust. Важка робота прибрана з полю зору, щоб ви рухалися швидко, і так само легко повертається, коли настав час масштабуватися.',
    'pillars.1.title': 'Батарейки в комплекті',
    'pillars.1.copy':
      'Сервіси, дані, листи, фонові завдання, таски та CLI, який керує всім цим — у коробці з першого дня.',
    'pillars.2.title': 'Rails — це чудово',
    'pillars.2.copy':
      'Loco наслідує Rails. Ну, ми це сказали — його концепції, дбайливо адаптовані до ідіоматичного Rust.',
    'pillars.3.title': 'Доставляйте з упевненістю',
    'pillars.3.copy':
      'Безапеляційно оптимізовано для розробника-одинаки. Складність і важка робота прибрані з полю зору.',
    'pillars.4.title': 'Масштабуйтеся, коли треба',
    'pillars.4.copy':
      'Розділіть, перебудуйте або використовуйте лише потрібні частини Loco. Зростайте без переписування.',
    'pillars.5.title': 'Будуйте поступово',
    'pillars.5.copy':
      'Просто сервіс. Або з базою даних. Або з фоновим воркером. Або з таскою. Використовуйте, що потрібно.',
    'pillars.6.title': 'Усе через тести',
    'pillars.6.copy':
      'Моделі, контролери, воркери — тестуйте весь застосунок із мінімальними зусиллями. Випускайте швидко, залишайтеся в безпеці.',

    'deck.head': 'Одна функція — кілька маленьких файлів.',
    'deck.prev': 'Попередня частина',
    'deck.next': 'Наступна частина',

    'footer.tagline': '© Loco — фреймворк для одного розробника на Rust',
    'footer.docs': 'Документація',
    'footer.github': 'GitHub',
    'footer.discord': 'Discord',
    'footer.blog': 'Блог',

    'home.title': 'Loco — фреймворк для одного розробника на Rust',
    'home.description': 'Loco — фреймворк для одного розробника на Rust.',

    'privacy.title': 'Політика конфіденційності',
    'privacy.description': 'Ми не використовуємо cookies і не збираємо персональні дані.',
    'privacy.tldr': 'Коротко',
    'privacy.tldrText': 'Ми не використовуємо cookies і не збираємо персональні дані.',
    'privacy.visitors': 'Відвідувачі сайту',
    'privacy.visitors1': 'Не збирається жодна персональна інформація.',
    'privacy.visitors2': 'В браузері не зберігається жодна інформація.',
    'privacy.visitors3': 'Жодна інформація не передається третім особам і не продається їм.',
    'privacy.visitors4': 'Жодна інформація не передається рекламним компаніям.',
    'privacy.visitors5':
      'Жодна інформація не видобувається для персональних чи поведінкових трендів.',
    'privacy.visitors6': 'Жодна інформація не монетизується.',
    'privacy.contact': 'Звʼяжіться з нами',
    'privacy.contactText': 'якщо у вас є запитання.',
    'privacy.effective': 'Чинно з 1 травня 2021 року',
  },
};

export type MsgKey = keyof (typeof dictionaries)['en'];

/**
 * Build a `t()` lookup bound to one locale. Falls back to English for any
 * key a translation is missing, so new UI never renders `undefined`.
 */
export function t(locale: Locale): (key: MsgKey) => string {
  const dict = dictionaries[locale] ?? dictionaries.en;
  const fallback = dictionaries.en;
  return (key) => dict[key] ?? fallback[key];
}

/** The locale of a URL path: `/uk/...` is Ukrainian, everything else English. */
export function localeFromPath(pathname: string): Locale {
  return pathname.startsWith('/uk/') || pathname === '/uk' ? 'uk' : 'en';
}

/**
 * Rewrite a root-relative path for the given locale. `/docs/x/` becomes
 * `/uk/docs/x/` for Ukrainian and stays `/docs/x/` for English.
 * External URLs and `#`/mailto are passed through untouched.
 */
export function localizedPath(path: string, locale: Locale): string {
  if (locale === 'en') return path;
  if (/^(https?:|#|mailto:)/.test(path)) return path;
  // Root becomes /uk/ (trailing slash required — the site builds with
  // trailingSlash: 'always', so a bare /uk would 404).
  return `/uk${path === '/' ? '/' : path}`;
}

/** Date formatting per locale (Ukrainian month names via Intl). */
export function formatDate(date: Date, locale: Locale): string {
  return date.toLocaleDateString(locale === 'uk' ? 'uk-UA' : 'en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}
