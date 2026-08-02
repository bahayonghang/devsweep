# VitePress Bilingual Site Research

## Question

How can the DevSweep documentation site provide maintainable English and
Simplified Chinese content using the requested VitePress dependency?

## Findings

- VitePress 1.6.4 requires Node.js 18 or later. The local Node.js 25.9.0
  runtime satisfies that prerequisite.
- VitePress treats `docs/` as the project root when commands use
  `vitepress <command> docs`. Its configuration resolves from
  `docs/.vitepress/config.ts` and supports TypeScript with ESM imports.
- The built-in `locales` configuration maps the root locale and language
  subdirectories to labels, language attributes, and locale links. A root
  English tree plus a `docs/zh/` Chinese tree does not require custom Vue code
  or server-side redirects.
- Locale-specific `themeConfig` supports separate navigation and sidebar labels
  while sharing the default theme and global settings.
- The recommended scripts are `vitepress dev docs`, `vitepress build docs`,
  and `vitepress preview docs`. The default build output and dev cache are
  `docs/.vitepress/dist` and `docs/.vitepress/cache` respectively.
- VitePress supports `srcExclude` globs, allowing the site to exclude
  repository-internal `docs/agents/**` files from generated public routes.

## Implementation Consequences

- Use a root `package.json` with `"type": "module"`, npm scripts, and the
  VitePress development dependency. Commit the generated npm lockfile.
- Configure `locales.root` for English and `locales.zh` for Simplified Chinese,
  with `/zh/` as the Chinese entry point.
- Store English source pages at the existing root paths and their Chinese
  counterparts below `docs/zh/`. Keep page paths mirrored so language switching
  is predictable and reviewable.
- Add `node_modules/`, `docs/.vitepress/cache/`, and `docs/.vitepress/dist/` to
  `.gitignore`.

## Sources

- VitePress 1.6.4 Internationalization guide:
  <https://raw.githubusercontent.com/vuejs/vitepress/v1.6.4/docs/en/guide/i18n.md>
- VitePress 1.6.4 Getting Started guide:
  <https://raw.githubusercontent.com/vuejs/vitepress/v1.6.4/docs/en/guide/getting-started.md>
- VitePress 1.6.4 Site Config reference:
  <https://raw.githubusercontent.com/vuejs/vitepress/v1.6.4/docs/en/reference/site-config.md>
- npm registry query: `npm view vitepress@1.6.4 engines --json`.

## Date

Research performed 2026-08-02.
