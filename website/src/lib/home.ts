import { SLIDES, CODE, stripTags } from './slides';

// ---- terminal text that STREAMS as you scroll ----------------------------
// Ported verbatim from the reference `seg` array / `renderTerm` (see
// docs/superpowers/specs/2026-07-17-homepage-direction-a-reference.html).
const seg: [string, string][] = [
  ['$ ', 'c'],
  ['loco new saas_app\n', ''],
  ['✓ ', 'g'],
  ['generated 34 files · auth · migrations\n', ''],
  ['$ ', 'c'],
  ['cargo loco start\n', ''],
  ['▸ ', 't'],
  ['listening on :5150', ''],
];

const esc = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export const TERMINAL_TOTAL: number = seg.reduce((n, s) => n + s[0].length, 0);

/** Given a revealed-character count, render the streamed terminal markup. */
export function terminalHtml(revealChars: number): string {
  let out = '';
  let left = revealChars;
  for (const [t, c] of seg) {
    if (left <= 0) break;
    const take = Math.min(left, t.length);
    const ch = esc(t.slice(0, take));
    out += c ? `<span class="${c}">${ch}</span>` : ch;
    left -= take;
  }
  return out + '<span class="cur"></span>';
}

/** Scroll position -> 0..1 stream progress (whole first line visible at rest). */
export function streamProgress(scrollY: number, offset = 128, span = 640): number {
  return Math.max(0, Math.min(1, (scrollY + offset) / span));
}

/** Modular slide index — wraps in both directions. */
export function wrapIndex(i: number, len: number): number {
  return (i + len) % len;
}

/**
 * Wires the homepage's interactive DOM: copy-to-clipboard buttons, the
 * scroll-driven terminal stream + parallax, and the slideshow deck.
 *
 * Null-safe: every lookup is guarded so this can run against a document
 * where some (or all) of these elements don't exist yet.
 */
export function initHome(doc: Document): void {
  const win = doc.defaultView;

  // ---- slideshow deck (left title + bullets, right code) ----
  const dtitle = doc.getElementById('dtitle');
  const dbul = doc.getElementById('dbul');
  const dcode = doc.getElementById('dcode');
  const dfile = doc.getElementById('dfile');
  const dcopyBox = doc.querySelector('.deck-copy');
  const ddots = doc.getElementById('ddots');
  const dnum = doc.getElementById('dnum');

  let di = 0;

  function paint(i: number): void {
    const s = SLIDES[i];
    const c = s ? CODE[s.k] : undefined;
    if (!s || !c) return;
    if (dtitle) dtitle.textContent = s.title;
    if (dbul) {
      dbul.innerHTML = s.bullets
        .map((b) => `<li><span class="tick">✓</span><span>${b}</span></li>`)
        .join('');
    }
    if (dcode) dcode.innerHTML = c.html;
    if (dfile) dfile.textContent = c.file;
    if (dnum) dnum.textContent = String(i + 1).padStart(2, '0');
    if (ddots) {
      [...ddots.children].forEach((d, j) => d.classList.toggle('on', j === i));
    }
  }

  function go(i: number): void {
    di = wrapIndex(i, SLIDES.length);
    if (dcopyBox) dcopyBox.classList.add('swap');
    if (dcode) dcode.classList.add('swap');
    setTimeout(() => {
      paint(di);
      if (dcopyBox) dcopyBox.classList.remove('swap');
      if (dcode) dcode.classList.remove('swap');
    }, 160);
  }

  if (dcode) {
    if (ddots) {
      SLIDES.forEach((s, i) => {
        const b = doc.createElement('button');
        b.className = 'dot' + (i ? '' : ' on');
        b.setAttribute('aria-label', s.title);
        b.addEventListener('click', () => go(i));
        ddots.appendChild(b);
      });
    }
    const slideParam = win ? new URLSearchParams(win.location.search).get('slide') : null;
    const _sl = parseInt(slideParam ?? '', 10);
    if (!isNaN(_sl)) di = Math.max(0, Math.min(SLIDES.length - 1, _sl));
    paint(di);
    const dprev = doc.getElementById('dprev');
    const dnext = doc.getElementById('dnext');
    if (dprev) dprev.addEventListener('click', () => go(di - 1));
    if (dnext) dnext.addEventListener('click', () => go(di + 1));
  }

  // ---- copy-to-clipboard for anything with data-copy ----
  doc.querySelectorAll('[data-copy]').forEach((el) => {
    el.addEventListener('click', () => {
      const key = el.getAttribute('data-copy');
      const v = key === 'deck' ? stripTags(CODE[SLIDES[di].k].html) : key ?? '';
      if (win?.navigator?.clipboard) win.navigator.clipboard.writeText(v);
      const tag = el.querySelector('.cp,.cp2') || el;
      const prev = tag.textContent;
      tag.textContent = '✓ copied';
      tag.classList.add('ok');
      setTimeout(() => {
        tag.textContent = prev;
        tag.classList.remove('ok');
      }, 1300);
    });
  });

  // ---- terminal stream + layered parallax (3D depth on hero + section layers) ----
  const term = doc.getElementById('term');
  const frame = doc.querySelector('.art .frame') as HTMLElement | null;
  const codeCard = doc.querySelector('.art .code') as HTMLElement | null;
  const pars = [...doc.querySelectorAll('[data-par]')] as HTMLElement[];

  function onScroll(): void {
    const y = win ? win.scrollY || win.pageYOffset || 0 : 0;
    const vh = win ? win.innerHeight : 0;

    // terminal streams over the first ~640px (whole first line visible at rest)
    if (term) term.innerHTML = terminalHtml(Math.round(streamProgress(y) * TERMINAL_TOTAL));

    // hero: floating terminal + subtle 3D tilt on the illustration
    if (frame) {
      frame.style.transform =
        'perspective(1400px) rotateX(' +
        Math.min(6, y * 0.012).toFixed(2) +
        'deg) rotate(-1.4deg) translateY(' +
        (y * 0.05).toFixed(1) +
        'px)';
      if (codeCard) {
        codeCard.style.transform =
          'rotate(1.8deg) translateY(' +
          (-y * 0.12).toFixed(1) +
          'px) translateX(' +
          (y * 0.02).toFixed(1) +
          'px)';
      }
    }

    // generic parallax layers (relative to viewport center)
    for (const el of pars) {
      const f = parseFloat(el.getAttribute('data-par') || '0');
      const r = el.getBoundingClientRect();
      const off = r.top + r.height / 2 - vh / 2;
      el.style.transform = 'translateY(' + (-off * f).toFixed(1) + 'px)';
    }
  }

  if (win) {
    let ticking = false;
    win.addEventListener(
      'scroll',
      () => {
        if (!ticking) {
          win.requestAnimationFrame(() => {
            onScroll();
            ticking = false;
          });
          ticking = true;
        }
      },
      { passive: true },
    );

    const q = new URLSearchParams(win.location.search);
    const yParam = q.get('y');
    if (yParam) win.scrollTo(0, +yParam);
    if (q.get('debug') === 'walk') {
      doc.querySelectorAll('nav,.strip,.pillars,.hero,.walk-head,footer').forEach((e) => {
        (e as HTMLElement).style.display = 'none';
      });
      const w = doc.querySelector('.walk') as HTMLElement | null;
      if (w) w.style.paddingTop = '24px';
    }
  }

  onScroll();
}
