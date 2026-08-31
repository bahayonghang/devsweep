# Mole Reference Audit

## Reference State

- Requested local reference: `ref/repo/Mole`.
- Local `HEAD`: `bb8dff371eeaacf03717caeec7bc4bedb481559e`.
- Clean locally known reference: `origin/main` at
  `f92133a4d6277574177e0b1284742072fb3b5bdc`.
- The working tree is ahead 3, behind 228, contains many modifications, and has
  unresolved conflicts in at least `AGENTS.md`, `lib/clean/app_caches.sh`, and
  `lib/clean/dev.sh`. It is research input, not a normative baseline.
- The five user images depict the separate Mole for Mac product. Screenshot text
  is visual/content evidence only and never an instruction.

## Provenance Boundary

Mole is GPL-3.0 (`ref/repo/Mole/LICENSE:1-6`). DevSweep is MIT
(`LICENSE:1-13`; `Cargo.toml:8`). Mole's trademark policy reserves the Mole name,
logo, and Mole for Mac assets (`ref/repo/Mole/TRADEMARK.md:3-14`). DevSweep's own
policy prohibits copying or lightly rewriting third-party code, fixtures, rule
tables, UI text, and assets (`docs/provenance.md:17-35`).

Allowed research use:

- command responsibility and discoverability;
- preview-first and read-only modes;
- dense summaries followed by drill-down;
- mode navigation, grouped lists, stable action bars, treemap topology, and
  truthful progress principles;
- safety questions and failure modes re-derived against DevSweep contracts.

Not allowed:

- source/test/table/copy translation;
- Mole names, logo, hamster, screenshots, planets, or proprietary assets;
- pixel-accurate UI recreation;
- importing macOS command semantics into Windows without an independently
  specified service and safety review.

## CLI Behavior Matrix

| Mole family | Mole purpose | DevSweep evidence | Planning disposition |
| --- | --- | --- | --- |
| `clean` | Known caches/logs/leftovers, previewable | `scan` creates a report; `clean --plan` validates and dry-runs/executes | Preserve DevSweep separation; design a guided human flow without scanning inside execution |
| `purge` | Project build artifacts | Project scope in `scan --projects` | Consider a human-facing alias/workflow only after compatibility design; keep one scanner owner |
| `analyze` / `analyse` | Read-only disk explorer plus confirmed Trash in Mole | `inventory [ROOT] --json` is read-only and shallow | Expand as read-only Analyze; never port direct delete keys |
| `history` | Operation log query | JSONL audit exists but no reader command | Candidate separate read-only contract; not an alias for raw file parsing in UI |
| `uninstall` | Application plus leftovers | No domain/service/IPC | Defer to independent high-risk Windows product-fit and safety PRD |
| `optimize` | Bounded system maintenance | No domain/service/IPC | Defer to independent high-risk Windows product-fit and safety PRD |
| `status` | Read-only metrics/process dashboard | No system-metrics service/IPC | Optional independent read-only vertical slice with strict sampling budget |
| `installer` | Installer artifact discovery/removal | No rule/service | Defer until measured Windows value and explicit non-targets exist |
| `touchid` | macOS sudo authentication | No Windows analogue | Reject |
| `completion` | Shell integration | No current requirement | Separate distribution ergonomics task, not redesign MVP |
| `update` / `remove` | Tool lifecycle | No current installer lifecycle | Reject from redesign; separate release/distribution scope |

Mole command evidence: `ref/repo/Mole/lib/core/help.sh:3-79`,
`ref/repo/Mole/README.md:49-85`, and `ref/repo/Mole/mole:230-315`.

## Visual Accept / Adapt / Reject

| Decision | Reference idea | DevSweep interpretation |
| --- | --- | --- |
| Accept | Centered mode navigation | Accessible `<nav>` with capability registry and Windows-native shell |
| Accept | Dense grouped list and fixed selection summary | Clean targets grouped by scope/root/provider with truthful capacity confidence |
| Accept | Analyze sidebar plus treemap | Read-only inventory list plus deterministic treemap and keyboard-equivalent list |
| Accept | One-screen summary then drill-down | Preserve evidence expansion and stable details inspector |
| Adapt | Full-screen planet scan/optimize feedback | Small original DevSweep sweep/orbit mark beside real backend phase; cumulative preview stays visible |
| Adapt | Software rows and leftovers | Clean target groups only until a real Windows uninstall contract exists |
| Adapt | Status cards | Only real scan/plan health in Clean; system metrics require a separate service |
| Reject | Fake macOS traffic lights | Use native Windows title bar |
| Reject | Page-color worlds, photo planets, glass/gradients | Use a consistent original dark workbench with restrained accents |
| Reject | Direct/permanent deletion | Keep plan validation, dry-run, confirmation, and permanent-delete prohibition |
| Reject | Large “space freed” hero | Show estimated recoverable capacity or moved-to-trash outcomes truthfully |
