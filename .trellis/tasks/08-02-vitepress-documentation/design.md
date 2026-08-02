# Design: VitePress Bilingual Documentation Site

## Boundary

This is one implementation task. The VitePress package, its bilingual routing,
the public pages, and `just docs` form one buildable documentation surface; no
part can be independently verified without the others.

The work adds documentation tooling and content only. It does not change Rust
runtime behavior, plan validation, cleanup authorization, or CI topology.

## Site Structure

English uses the root locale and Chinese uses the `zh` locale:

```text
docs/
  .vitepress/config.ts
  index.md
  guide/
    getting-started.md
    safety-model.md
    tui.md
    scan.md
    inventory.md
    clean.md
    protection.md
  reference/
    cli.md
    plan-and-report.md
    rules.md
  ci.md
  provenance.md
  zh/
    index.md
    guide/...
    reference/...
    ci.md
    provenance.md
```

Every public English page has a Chinese counterpart at the equivalent `/zh/`
path. The root-level `ci.md` and `provenance.md` retain their existing paths so
repository links do not break. `docs/agents/**` remains in the repository but
is excluded from VitePress sources and navigation.

## VitePress Configuration

`docs/.vitepress/config.ts` will use `defineConfig` from VitePress and declare:

- `root` as an English (`en-US`) locale and `zh` as a Simplified Chinese
  (`zh-CN`) locale with `/zh/` as its entry link;
- localized title, description, navigation, sidebar, and language-switcher
  labels in `themeConfig.locales`;
- a default-theme edit link pattern targeting the repository's `main` branch;
- `srcExclude: ['agents/**']` to keep agent-operational Markdown out of the
  generated public site;
- no deployment-specific `base` setting, because deployment is out of scope.

The default theme is sufficient for this operational documentation surface. No
custom Vue components, client analytics, external search service, or generated
visual asset is needed.

## Content Contract

The landing pages describe DevSweep as a safety-first cleanup planner and link
to the primary workflows. Guide pages explain the user journey: inspect,
generate a plan, review it, dry-run it, and explicitly execute a saved plan.

The content will cover all public CLI subcommands from `src/cli.rs`:

- `tui` for interactive review;
- `scan` with project/global scope, JSON output, and targeted re-estimation;
- `inventory` as an inspect-only capacity report;
- `clean` with saved-plan validation, dry-run, explicit execution, and audit
  log behavior;
- `protect add`, `protect remove`, and `protect list`;
- `rules` as an inspectable rule catalog.

Reference pages will explain stable concepts rather than reproduce every Rust
internal field: scan report and plan versions, scan health and diagnostics,
known versus partial size totals, cleanup target evidence/risk/intent, and the
boundary between a user-provided plan document and executable actions rebuilt
from the trusted rule registry. Both languages must preserve the same safety
claims and command semantics.

## Tooling and Generated Files

The root `package.json` uses ESM and defines `docs:dev`, `docs:build`, and
`docs:preview` scripts. `vitepress` is the single new development dependency;
`package-lock.json` captures its resolved install graph. The root `.gitignore`
will ignore `node_modules/`, `docs/.vitepress/cache/`, and
`docs/.vitepress/dist/`.

`just docs` delegates to `npm run docs:dev`, matching the existing `just dev`
convention for interactive local workflows. The build command stays in npm so
CI or future deployment can invoke it without starting a server.

The development command binds VitePress explicitly to `127.0.0.1`. npm audit
currently reports unfixable Vite/VitePress development-server advisories for
the latest stable VitePress 1.6.4 dependency chain, so the local-only binding
reduces network exposure without claiming to remove the upstream risk.

## Compatibility and Rollback

The site has no effect on the compiled Rust binary. Removing the package files,
VitePress configuration, and public pages restores the prior documentation
layout; retained `docs/ci.md` and `docs/provenance.md` keep their source paths
throughout the change. No data migration, cleanup execution, or external
publication is involved.
