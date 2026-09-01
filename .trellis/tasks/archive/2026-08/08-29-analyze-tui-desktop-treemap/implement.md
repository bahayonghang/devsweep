# Implement - Analyze TUI and Desktop Treemap

Start only after later approval, the shell/spec update, and the frozen Analyze
core/IPC task pass. This task performs no filesystem reads.

## 1. Add pure bounded views

1. Add immutable current-directory selectors, 200-row paging, breadcrumbs,
   search/sort, and focus anchors.
2. Add the pure squarified layout with 512 known-child tiles plus one `Other`;
   unknown bytes never receive area.
3. Test deterministic rounding, equality/extremes, pagination, `Other` byte/count
   reconciliation, and no cleanup action.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli tui::modes::analyze
rtk just desktop-web-check
```

Rollback point: remove TUI/Desktop Analyze modules and mode registration without
changing the frozen snapshot DTO.

## 2. Wire state, IPC, and accessibility

1. Consume generated DTOs through mode-local reducers; reject stale ids/events.
2. Link rectangles to the paged DOM list/tree and keyboard drill-down/up; keep
   text/table alternatives canonical.
3. Cancel and join through the shell on navigation/close, then clear ephemeral
   view state.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli tui::modes::analyze
rtk just desktop-web-check
rtk just desktop-build
```

## 3. Fixed render and native gates

1. Run five warm-up navigations, then 30 recorded navigations on
   `analysis-250k-v1`; capture layout time, React commit, DOM count, and memory.
2. Require <=513 rectangles, <=900 DOM elements, p95 layout <=50 ms, and p95
   commit <=100 ms. Any miss returns to planning rather than raising the cap.
3. Record both languages, 390/800/1024/1440 widths, 100/125/150/200% scaling,
   keyboard/screen-reader alternatives, high contrast, reduced motion, partial/
   unknown states, and cancel-on-leave.

```powershell
rtk git diff --check
rtk just ci
```

Stop for any new rendering dependency, canvas-only interaction, snapshot schema
change, hidden unknown capacity, or shell/spec change.
