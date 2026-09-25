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

// Ukrainian mirrors of the blog — same filenames/ids as `blog`, translated
// frontmatter + body. Pages under /uk/blog/ read from this collection.
const blogUk = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/blog-uk' }),
  schema: blog.schema,
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

// Ukrainian mirrors of the casts — same filenames/ids as `casts`.
const castsUk = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/casts-uk' }),
  schema: casts.schema,
});

const authors = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/authors' }),
  schema: z.object({
    name: z.string(),
    description: z.string(),
  }),
});

// Ukrainian author bios — same filenames/ids as `authors`.
const authorsUk = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/authors-uk' }),
  schema: authors.schema,
});

export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  blog,
  blogUk,
  casts,
  castsUk,
  authors,
  authorsUk,
};
