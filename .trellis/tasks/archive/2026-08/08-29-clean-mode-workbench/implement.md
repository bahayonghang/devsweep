# Implement - Clean Mode Workbench

Start only after later explicit approval and successful completion of the CLI,
shell, and bounded-sizing gates. The paused sizing child is not resumed by this
plan.

## 1. Preserve and expose the Clean authority chain

1. Replace the legacy audit writer with the fixed-path, locked, durable,
   redacted Clean V1 writer before exposing any new mutation. Add exact bytes,
   unknown/corrupt preservation, lock/flush failure, legacy non-discovery, and
   History handoff fixtures, including the closed protection-mutation variant.
2. Add the Clean handler and bilingual `presentation/clean.rs` renderer for the
   exact frozen grammar without editing `application/cli.rs` or either module
   root.
3. Trace report -> exact selection -> saved untrusted plan -> live preview ->
   digest -> confirm -> execution through core, CLI, Tauri, TUI, and desktop.
4. Add hostile/stale/reparse/protection/inspect-only/cancel/partial fixtures
   before changing presentation.

Focused validation:

```powershell
rtk cargo test -p devsweep-core scan
rtk cargo test -p devsweep-core plan
rtk cargo test -p devsweep-core execution
rtk cargo test -p devsweep-core audit
rtk cargo test -p devsweep-cli clean
```

Rollback point: remove the Clean adapter and generated fixtures together; retain
the accepted V1 audit bytes/readability and do not revert accepted sizing.

## 2. Build mode-local TUI/Desktop state and presentation

1. Add mode-local reducers/effects; never move domain state into the shell.
2. Implement dense rows, evidence details, safe-scope selection, sticky summary,
   preview/confirm/execution states, and operation-id stale-event rejection.
3. Regenerate DTO types and add English/Chinese state/error fixtures.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli tui::modes::clean
rtk just desktop-web-check
```

Rollback point: unregister `desktop/src/modes/clean` and TUI mode modules as one
unit; the prior Clean surface remains the fallback until final integration.

## 3. Full and native validation

```powershell
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
rtk just ci
```

- On Windows, record both languages at 390/800/1024/1440 CSS px and
  100/125/150/200% scaling, keyboard-only selection, long paths, partial/error,
  cancellation, dry run, and disposable trash execution. Display scaling is
  changed only by the user. Trash evidence may use only a newly created,
  task-owned temporary fixture whose exact paths and saved plan are recorded;
  permanent deletion remains disabled.
- Confirm rescan/selection changes synchronously invalidate preview authority,
  inspect-only entries never select, no UAC appears, and trash copy never claims
  space is already freed.
- Stop and return to the owning task if sizing evidence is not accepted, the
  frozen CLI must change, or a shell/spec change is needed.
