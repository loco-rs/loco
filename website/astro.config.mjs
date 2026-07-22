import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';

// A compact Shiki/VS-Code-shaped theme carrying the warm-dark code palette
// from the homepage code windows and the docs reference
// (docs/superpowers/specs/2026-07-17-docs-starlight-reference.html: `.cb`
// bg #1b1512, bar #241d19, border #2a2320, `.k`/`.s`/`.c`/`.t`/`.fn` token
// colors). Code blocks intentionally stay this warm-dark in BOTH site
// themes — see starlight.css's `--ec-*` overrides for the surrounding
// frame chrome, which must agree with these `colors` for a seamless block.
const locoCodeTheme = {
  name: 'loco-warm-dark',
  type: 'dark',
  colors: {
    'editor.background': '#1b1512',
    'editor.foreground': '#e9ddcf',
  },
  tokenColors: [
    { settings: { foreground: '#e9ddcf' } },
    {
      scope: ['comment'],
      settings: { foreground: '#7d7266', fontStyle: 'italic' },
    },
    {
      scope: ['string', 'string.quoted', 'constant.character', 'markup.inline.raw'],
      settings: { foreground: '#c3e88d' },
    },
    {
      scope: [
        'keyword',
        'keyword.control',
        'storage.type',
        'storage.modifier',
        'constant.language',
      ],
      settings: { foreground: '#ff8f6b' },
    },
    {
      scope: ['entity.name.function', 'support.function', 'meta.function-call'],
      settings: { foreground: '#ffd479' },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'support.type',
        'support.class',
        'meta.path',
      ],
      settings: { foreground: '#82aaff' },
    },
    {
      scope: ['constant.numeric', 'constant.other'],
      settings: { foreground: '#f5b784' },
    },
    {
      scope: ['punctuation', 'meta.brace', 'punctuation.definition'],
      settings: { foreground: '#a89a8c' },
    },
  ],
};

export default defineConfig({
  site: 'https://loco.rs',
  // The old Zola-built docs used trailing slashes throughout; keep URLs
  // stable across the migration.
  trailingSlash: 'always',
  // Prefetch internal links on hover so docs sidebar navigation loads the
  // target page before the click resolves — kills the "content loading" flash
  // of a cold full-page (MPA) navigation.
  prefetch: { prefetchAll: true, defaultStrategy: 'hover' },
  integrations: [
    tailwind({ applyBaseStyles: false }),
    starlight({
      title: 'Loco',
      customCss: ['./src/styles/starlight.css'],
      // The docs Header override (below) renders the marketing brand + a
      // VersionBadge itself, so Starlight's default SiteTitle isn't used.
      components: {
        // Docs header mirrors the marketing top bar (brand, centered links,
        // star, circular theme toggle, CTA); see the component for rationale.
        Header: './src/components/starlight/Header.astro',
      },
      // Diátaxis groups. `How-to guides` holds 35 pages: left as one flat
      // `autogenerate` it renders as an undifferentiated wall, so it's
      // hand-organized into task-themed subgroups below. Entries reference
      // the EXISTING flat slugs (`docs/how-to/<name>`) — no files move, so
      // every published URL stays identical (URL parity preserved).
      sidebar: [
        { label: 'Tutorials', autogenerate: { directory: 'docs/tutorials' } },
        {
          label: 'How-to guides',
          items: [
            { slug: 'docs/how-to' },
            {
              label: 'Data & models',
              items: [
                { slug: 'docs/how-to/add-model' },
                { slug: 'docs/how-to/query-data' },
                { slug: 'docs/how-to/paginate' },
                { slug: 'docs/how-to/seed-data' },
                { slug: 'docs/how-to/load-data' },
                { slug: 'docs/how-to/multi-database' },
              ],
            },
            {
              label: 'Web layer',
              items: [
                { slug: 'docs/how-to/add-controller' },
                { slug: 'docs/how-to/validate-requests' },
                { slug: 'docs/how-to/respond-formats' },
                { slug: 'docs/how-to/render-views' },
                { slug: 'docs/how-to/handle-errors' },
                { slug: 'docs/how-to/add-middleware' },
                { slug: 'docs/how-to/serve-assets' },
                { slug: 'docs/how-to/websockets' },
              ],
            },
            {
              label: 'Background work',
              items: [
                { slug: 'docs/how-to/add-worker' },
                { slug: 'docs/how-to/choose-queue-backend' },
                { slug: 'docs/how-to/schedule-jobs' },
                { slug: 'docs/how-to/write-task' },
                { slug: 'docs/how-to/send-email' },
              ],
            },
            {
              label: 'Auth & security',
              items: [
                { slug: 'docs/how-to/jwt-auth' },
                { slug: 'docs/how-to/api-key-auth' },
                { slug: 'docs/how-to/jwt-locations' },
                { slug: 'docs/how-to/hash-passwords' },
              ],
            },
            {
              label: 'Testing',
              items: [
                { slug: 'docs/how-to/request-tests' },
                { slug: 'docs/how-to/model-tests' },
                { slug: 'docs/how-to/fixtures-snapshots' },
              ],
            },
            {
              label: 'Ops & configuration',
              items: [
                { slug: 'docs/how-to/configure-storage' },
                { slug: 'docs/how-to/use-cache' },
                { slug: 'docs/how-to/configure-logging' },
                { slug: 'docs/how-to/connect-over-tls' },
                { slug: 'docs/how-to/deploy' },
              ],
            },
            {
              label: 'Generators & tooling',
              items: [
                { slug: 'docs/how-to/use-generators' },
                { slug: 'docs/how-to/override-templates' },
                { slug: 'docs/how-to/run-doctor' },
              ],
            },
          ],
        },
        { label: 'Reference', autogenerate: { directory: 'docs/reference' } },
        { label: 'Explanation', autogenerate: { directory: 'docs/explanation' } },
        { label: 'Extras', autogenerate: { directory: 'docs/extras' } },
        { label: 'Resources', autogenerate: { directory: 'docs/resources' } },
      ],
      expressiveCode: {
        themes: [locoCodeTheme],
        // A single always-dark theme by design (see locoCodeTheme comment
        // above) — there's nothing to switch between, and Starlight's own
        // light/dark UI colors would otherwise leak into the code chrome.
        useStarlightDarkModeSwitch: false,
        useStarlightUiThemeColors: false,
      },
    }),
    sitemap(),
  ],
});
