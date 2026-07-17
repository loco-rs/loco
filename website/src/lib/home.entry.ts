// Client entry point: wires up the homepage's interactive DOM (terminal
// stream, parallax, deck, copy buttons, scroll reveals) once the document
// is ready. Kept as its own tiny module (rather than inlining the call in
// index.astro) so Astro bundles a single small script with no framework
// runtime — see Task 8's a11y/perf pass.
import { initHome } from './home';

function start(): void {
  initHome(document);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', start);
} else {
  start();
}
