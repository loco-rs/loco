import { defineCollection } from 'astro:content';
// `z` re-exported from `astro:content` is deprecated; Astro's own type
// declaration points here instead.
import { z } from 'astro/zod';
import { glob } from 'astro/loaders';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

const blog = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/blog' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    pubDate: z.coerce.date(),
    updatedDate: z.coerce.date().optional(),
    authors: z.array(z.string()),
  }),
});

const casts = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/casts' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    pubDate: z.coerce.date(),
    updatedDate: z.coerce.date().optional(),
    authors: z.array(z.string()),
    episode: z.string(),
    youtube: z.string(),
  }),
});

const authors = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/authors' }),
  schema: z.object({
    name: z.string(),
    description: z.string(),
  }),
});

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  blog,
  casts,
  authors,
};
