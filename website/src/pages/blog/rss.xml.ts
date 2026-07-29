// RSS 2.0 feed for the blog, served at the old Zola feed URL `/blog/rss.xml`
// (preserving URL parity — see scripts/url-parity-blog.mjs).
import rss from '@astrojs/rss';
import type { APIContext } from 'astro';
import { getCollection } from 'astro:content';

export async function GET(context: APIContext) {
  const posts = (await getCollection('blog')).sort(
    (a, b) => b.data.pubDate.valueOf() - a.data.pubDate.valueOf()
  );

  return rss({
    title: 'Loco Blog',
    description: 'Notes, tutorials, and updates from the Loco community — the one-person framework for Rust.',
    site: context.site!,
    items: posts.map((post) => ({
      title: post.data.title,
      description: post.data.description,
      pubDate: post.data.pubDate,
      link: `/blog/${post.id}/`,
    })),
  });
}
