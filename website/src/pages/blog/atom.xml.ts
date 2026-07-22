// Hand-built Atom 1.0 feed for the blog, served at the old Zola feed URL
// `/blog/atom.xml` (preserving URL parity — see
// scripts/url-parity-blog.mjs). @astrojs/rss only emits RSS 2.0, so this
// feed is assembled directly per the Atom 1.0 spec (RFC 4287).
import type { APIContext } from 'astro';
import { getCollection } from 'astro:content';

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

export async function GET(context: APIContext) {
  const site = context.site!;
  const posts = (await getCollection('blog')).sort(
    (a, b) => b.data.pubDate.valueOf() - a.data.pubDate.valueOf()
  );

  const feedUpdated = (posts[0]?.data.updatedDate ?? posts[0]?.data.pubDate ?? new Date()).toISOString();

  const entries = posts
    .map((post) => {
      const link = new URL(`/blog/${post.id}/`, site).href;
      const updated = (post.data.updatedDate ?? post.data.pubDate).toISOString();
      return `  <entry>
    <title>${escapeXml(post.data.title)}</title>
    <link href="${escapeXml(link)}" />
    <id>${escapeXml(link)}</id>
    <updated>${updated}</updated>
    <summary>${escapeXml(post.data.description)}</summary>
  </entry>`;
    })
    .join('\n');

  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Loco Blog</title>
  <subtitle>Notes, tutorials, and updates from the Loco community — the one-person framework for Rust.</subtitle>
  <link href="${new URL('/blog/atom.xml', site).href}" rel="self" />
  <link href="${new URL('/blog/', site).href}" />
  <id>${new URL('/blog/', site).href}</id>
  <updated>${feedUpdated}</updated>
  <author><name>Loco</name></author>
${entries}
</feed>
`;

  return new Response(xml, {
    headers: { 'Content-Type': 'application/xml' },
  });
}
