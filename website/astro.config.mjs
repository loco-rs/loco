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
  integrations: [
    tailwind({ applyBaseStyles: false }),
    starlight({
      title: 'Loco',
      customCss: ['./src/styles/starlight.css'],
      // Six Diátaxis groups. Content is migrated in a later task — until
      // then each `docs/<section>` directory is empty and its group
      // simply renders with no entries.
      sidebar: [
        { label: 'Tutorials', autogenerate: { directory: 'docs/tutorials' } },
        { label: 'How-to guides', autogenerate: { directory: 'docs/how-to' } },
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
