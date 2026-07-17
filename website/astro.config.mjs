import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://loco.rs',
  integrations: [
    tailwind({ applyBaseStyles: false }),
    starlight({
      title: 'Loco',
      // Docs content is migrated in a later plan; keep the route buildable.
      sidebar: [{ label: 'Docs', items: [{ label: 'Overview', slug: 'docs' }] }],
    }),
    sitemap(),
  ],
});
