import { describe, it, expect } from 'vitest';
import { JSDOM } from 'jsdom';
import { SLIDES, CODE, stripTags } from './slides';
import { terminalHtml, streamProgress, wrapIndex, TERMINAL_TOTAL, initHome } from './home';

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

describe('terminal streaming', () => {
  it('reveals nothing but a cursor at 0 chars', () => {
    expect(terminalHtml(0)).toBe('<span class="cur"></span>');
  });
  it('reveals the whole first line at the rest offset', () => {
    // (128/640)*TERMINAL_TOTAL rounds to the first prompt line
    const html = terminalHtml(Math.round((128/640)*TERMINAL_TOTAL));
    expect(html).toContain('loco new saas_app');
    expect(html).not.toContain('listening on');
  });
  it('streamProgress clamps to 0..1', () => {
    expect(streamProgress(-9999)).toBe(0);
    expect(streamProgress(99999)).toBe(1);
  });
});
describe('deck index', () => {
  it('wraps around both directions', () => {
    expect(wrapIndex(-1, 6)).toBe(5);
    expect(wrapIndex(6, 6)).toBe(0);
  });
});

describe('initHome DOM wiring', () => {
  function makeDoc(html: string): Document {
    return new JSDOM(html, { url: 'https://loco.rs/' }).window.document;
  }

  it('is a no-op on an empty document (null-safe)', () => {
    const doc = makeDoc('<!DOCTYPE html><html><body></body></html>');
    expect(() => initHome(doc)).not.toThrow();
  });

  it('paints the first deck slide on init', () => {
    const doc = makeDoc(`<!DOCTYPE html><html><body>
      <h2 id="dtitle"></h2>
      <ul id="dbul"></ul>
      <pre id="dcode"></pre>
      <span id="dfile"></span>
      <div class="deck-copy"></div>
      <div id="ddots"></div>
      <span id="dnum"></span>
      <button id="dprev"></button>
      <button id="dnext"></button>
    </body></html>`);
    initHome(doc);
    expect(doc.getElementById('dtitle')!.textContent).toBe(SLIDES[0].title);
    expect(doc.getElementById('dfile')!.textContent).toBe(CODE[SLIDES[0].k].file);
    expect(doc.getElementById('dnum')!.textContent).toBe('01');
    expect(doc.getElementById('ddots')!.children.length).toBe(SLIDES.length);
  });

  it('dnext advances the deck to the second slide', () => {
    const doc = makeDoc(`<!DOCTYPE html><html><body>
      <h2 id="dtitle"></h2>
      <ul id="dbul"></ul>
      <pre id="dcode"></pre>
      <span id="dfile"></span>
      <div class="deck-copy"></div>
      <div id="ddots"></div>
      <span id="dnum"></span>
      <button id="dprev"></button>
      <button id="dnext"></button>
    </body></html>`);
    initHome(doc);
    (doc.getElementById('dnext') as HTMLButtonElement).click();
    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(doc.getElementById('dtitle')!.textContent).toBe(SLIDES[1].title);
        resolve();
      }, 200);
    });
  });

  it('copy button copies a literal data-copy value without throwing', () => {
    const doc = makeDoc('<!DOCTYPE html><html><body><button data-copy="cargo install loco-cli">copy</button></body></html>');
    initHome(doc);
    const btn = doc.querySelector('[data-copy]') as HTMLButtonElement;
    expect(() => btn.click()).not.toThrow();
    expect(btn.textContent).toBe('✓ copied');
  });
});
