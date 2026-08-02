# Implementation Plan: VitePress Bilingual Documentation Site

## Preconditions

- Read the task PRD, design, and VitePress research before editing.
- Load the Trellis pre-development guidance and inspect the live CLI/source
  contracts again before writing command examples.
- Keep work scoped to documentation, npm tooling, ignore rules, and `justfile`.

## Steps

1. Add root npm tooling with ESM mode and `docs:dev`, `docs:build`, and
   `docs:preview` scripts. Install the pinned VitePress dependency and commit
   the generated `package-lock.json`.
2. Add generated npm and VitePress paths to `.gitignore` without changing the
   existing Rust-output rules.
3. Create `docs/.vitepress/config.ts` with English-root and Chinese-`/zh/`
   locales, localized navigation/sidebars, a language switcher, default-theme
   edit links, and the `docs/agents/**` source exclusion.
4. Create the English landing, guide, and reference pages. Preserve and expand
   the existing English CI/release and provenance pages as needed for site
   navigation. Use live CLI/source behavior as the authority for examples.
5. Create the Simplified Chinese pages at matching `docs/zh/` paths. Translate
   operational meaning rather than literal phrasing, and preserve command names,
   flags, file names, JSON fields, and safety semantics exactly.
6. Add `docs` to `justfile` so `just docs` invokes `npm run docs:dev`.
7. Review all internal links, the bilingual sidebar parity, and the safety
   language before validation.
8. Bind the development server to loopback and record any npm audit finding
   that lacks an upstream stable-version remediation.

## Validation

1. Run `npm ci` to prove the lockfile is installable from a clean dependency
   directory.
2. Run `npm run docs:build` and confirm VitePress generates both root and
   `/zh/` routes without broken internal links or build warnings.
3. Start `just docs`, verify the local server serves the English root and
   Chinese `/zh/` entry, then stop the server cleanly.
4. Run `git diff --check` and confirm generated npm/VitePress output remains
   ignored.
5. Run `just ci` for the repository's canonical Rust gate and inspect the final
   diff to ensure no runtime or safety-policy behavior changed.

## Review Gates

- Check each public command in `src/cli.rs` has a reachable bilingual page or
  section.
- Check that no example places `--execute` before a saved-plan review step or
  suggests permanent deletion or Docker cleanup.
- Check that plan/report pages distinguish observed, untrusted serialized data
  from the validated executable actions reconstructed by the program.
- Check that `docs/agents/**` is not generated as a public route.

## Rollback Point

If VitePress localization or build behavior proves incompatible, stop before
changing Rust code. Revert only the documentation-tooling files in this task
and retain the pre-existing Markdown pages at their original paths.
