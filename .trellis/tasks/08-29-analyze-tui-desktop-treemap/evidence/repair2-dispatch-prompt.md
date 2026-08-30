Active task: .trellis/tasks/08-29-analyze-tui-desktop-treemap

MINIMAL REPAIR (round 2 of the render-evidence loop; single-file scope). You are the trellis-implement sub-agent in repo D:\Documents\Code\Rust\Exp\devsweep (Windows, Git Bash shell). All standing constraints apply: no new dependencies, no commit/push, do not touch `.trellis/.gitignore`, `README.md`, `justfile`, other task directories, frozen caps/DTOs/CLI parser. Do not spawn sub-agents.

## The one defect
`desktop/src/modes/analyze/AnalyzePage.test.tsx` fails to load under Vitest. The previous fix reads the stylesheet with `readFileSync(new URL("./styles.css", import.meta.url), "utf8")`, but under this Vitest setup `import.meta.url` is not a `file:` URL, so the suite errors with `TypeError: The URL must be of scheme file` before any test runs (see `evidence/desktop-web-check-round3.log`: "Test Files 1 failed | 17 passed (18); Tests 115 passed (115)").

## The fix (exact scope)
Change ONLY the stylesheet-reading mechanism in `desktop/src/modes/analyze/AnalyzePage.test.tsx` so it resolves the checked-in `src/modes/analyze/styles.css` robustly under Vitest — e.g. resolve against the Vitest project root via `process.cwd()` (`resolve(process.cwd(), "src/modes/analyze/styles.css")`) or any mechanism that does not depend on `import.meta.url` being a `file:` URL and does not use `?raw`. Keep the accessibility-CSS assertions themselves unchanged. Do not touch any other file.

## Validation you CAN run in your sandbox
- `npm --prefix desktop run lint` must stay exit 0.
- `npm --prefix desktop run typecheck` must stay exit 0.
- You CANNOT run Vitest (`spawn EPERM` in your sandbox) — state that explicitly in your report; the main session will run `just desktop-web-check` and record the result.

## Evidence
Append a journal line to `.trellis/tasks/08-29-analyze-tui-desktop-treemap/implement.jsonl`. Save `lint`/`typecheck` logs under `evidence/` with a `repair2-` prefix. Report: the exact diff, both exit codes, and the explicit note that Vitest verification is delegated to the main session.
