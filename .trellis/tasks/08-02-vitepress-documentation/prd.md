# Create VitePress documentation site

## Goal

Publish a complete bilingual VitePress documentation site from `docs/` so
English- and Chinese-speaking developers can understand DevSweep's workflow,
commands, safety guarantees, machine-readable reports, and validation process
without reading Rust source.

## Background

- DevSweep 0.2.0 is a Rust CLI for safety-first developer cleanup planning and
  execution. Its public commands are `tui`, `scan`, `inventory`, `clean`,
  `protect`, and `rules`.
- `clean` is dry-run by default. Execution requires a saved plan plus
  `--execute`; permanent deletion and Docker cleanup are outside the current
  product behavior.
- Scan output uses versioned scan reports and cleanup-plan documents. Inventory
  reports are read-only and cannot be supplied to `clean` as a plan.
- Existing `docs/` files cover provenance, CI/release gates, and internal agent
  guidance, but no VitePress configuration, Node manifest, or documentation
  build command exists.
- Node.js 25.9.0 and npm 11.19.0 are available locally. VitePress 1.6.4
  supports Node.js 18 or later and provides built-in locale routing.

## Key Decisions

- The documentation site will provide equivalent English and Simplified Chinese
  page trees. English remains at the root URL; Simplified Chinese lives under
  `/zh/` with an explicit language switcher.
- `just docs` will start the VitePress development server. `npm run docs:build`
  remains the non-interactive production-build check.
- The site will use VitePress's default theme and built-in localization rather
  than a custom Vue theme or a third-party translation/search service.
- Existing English provenance and CI/release pages will remain reachable at
  their current source paths. Internal Trellis and agent-operational documents
  will not appear in public navigation or generated routes.

## Requirements

1. Add a VitePress site rooted at `docs/`, including a TypeScript site
   configuration, bilingual navigation and sidebars, an npm dependency manifest,
   and a lockfile for repeatable installs.
2. Create matching English and Simplified Chinese pages for getting started, the
   safety model, TUI use, scanning, inventory, cleanup-plan execution, path
   protection, rules, CLI reference, plan/report format, CI and release gates,
   and provenance.
3. Derive command examples and format explanations from live CLI and serialized
   model contracts. Preserve the safety boundary in both languages: scanning
   creates plans; cleanup remains dry-run unless an explicit saved plan and
   `--execute` are supplied; no page may imply permanent deletion or Docker
   cleanup.
4. Add `just docs` as the local interactive documentation command, and ensure
   cache, build output, and npm dependencies are not committed.
5. Keep the public information architecture focused on DevSweep users while
   retaining existing internal repository documentation unchanged.

## Acceptance Criteria

- [x] `docs/.vitepress/config.ts` configures English root and Simplified Chinese
  `/zh/` locales, a language menu, matching navigation, and matching sidebars.
- [x] Both language trees have a landing page and linked pages covering every
  public CLI command and the current safety model.
- [x] Command examples and plan/report explanations match `src/cli.rs`,
  `src/main.rs`, `src/model.rs`, `src/inventory.rs`, and the safety policy.
- [x] Existing provenance and CI/release material remains linked in English and
  has a semantically equivalent Chinese page.
- [x] `npm ci` followed by `npm run docs:build` completes successfully, and
  `just docs` starts the VitePress development workflow.
- [x] Generated VitePress cache/output and npm dependency directories remain
  ignored after a production documentation build.

## Verification Record

- `npm ci`, `npm run docs:build`, and `just ci` completed successfully after
  the final documentation review (221 Rust tests).
- The VitePress development server returned HTTP 200 for English and `/zh/`
  routes on a loopback-only host. Built output contains no `docs/agents/**`
  route, asset, or reference.
- `npm audit` reports three upstream, development-server-chain advisories with
  no available fix in the pinned stable VitePress dependency graph. The local
  development server binds explicitly to `127.0.0.1` as mitigation.
- Spec update assessment: no code-spec update is needed. This task adds public
  documentation tooling and does not change Rust behavior, CLI flags, or a
  cross-layer runtime contract.

## Out Of Scope

- Publishing to GitHub Pages or adding a documentation deployment workflow.
- Changing DevSweep's Rust behavior, cleanup policy, CLI flags, CI jobs, or
  release archive format.
- Adding permanent-delete or Docker-cleanup instructions.
- Translating or changing internal Trellis task artifacts and agent guidance.
