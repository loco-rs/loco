import { describe, it, expect } from 'vitest';
import { SLIDES, CODE, stripTags } from './slides';

describe('slides data', () => {
  it('has the six parts in order', () => {
    expect(SLIDES.map(s => s.k)).toEqual(['controller','model','worker','view','task','mailer']);
  });
  it('every slide has a matching real code entry with a filename and 3 bullets', () => {
    for (const s of SLIDES) {
      expect(CODE[s.k]).toBeTruthy();
      expect(CODE[s.k].file).toMatch(/^src\/.+\.rs$/);
      expect(s.bullets.length).toBe(3);
    }
  });
  it('code is real Loco: controller references the model and view', () => {
    const plain = stripTags(CODE.controller.html);
    expect(plain).toContain('articles::Model::latest');
    expect(plain).toContain('ArticleResponse');
    expect(plain).toContain('Routes::new()');
  });
  it('stripTags removes span markup but keeps code text', () => {
    expect(stripTags('<span class="k">use</span> loco;')).toBe('use loco;');
  });
});
