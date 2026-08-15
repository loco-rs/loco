/// <reference types="@astrojs/starlight/virtual" />

// `src/components/starlight/Header.astro` overrides Starlight's own header and
// mirrors it, including `import Search from 'virtual:starlight/components/Search'`
// — the way Starlight's `components/Header.astro` imports it. That module is
// declared in `virtual-internal.d.ts`, which the public `virtual` reference
// above does not pull in and which the package does not export by a stable
// specifier, so it is declared here rather than referenced by path.
declare module 'virtual:starlight/components/Search' {
  const Search: typeof import('@astrojs/starlight/components/Search.astro').default;
  export default Search;
}
