# TUI Internal Architecture Design

## Target Tree

```text
tui/
  mod.rs              run composition only
  terminal.rs         terminal lifecycle RAII
  display.rs          pure shared text/path/action/command presentation
  runtime/
    mod.rs            event loop, effect dispatch, cancellation registry
    services.rs       scan/inventory/clean traits and production adapters
    workers.rs        worker execution and WorkerEvent translation
  app/
    mod.rs            App state, constructor, root update, shared state types
    events.rs         UiEvent, Effect, WorkerEvent, JobId protocol
    input.rs          normal/filter/overlay/confirmation/quit keys
    selection.rs      visible rows, filters, grouping, cursor, selected bytes
    worker.rs         staged scan/inventory/clean event application
    jobs.rs           job transitions, logs, progress, frozen confirmation
  render/
    mod.rs            render_app, root layout, header/tabs/footer routing
    theme.rs          styles and risk/status tones
    targets.rs        dashboard/categories/targets/details/rules
    inventory.rs      read-only inventory view/details
    jobs.rs           jobs/logs/diagnostics view
    overlays.rs       confirmation, cleanup progress, help/error/quit modals
    format.rs         cell-width truncation and render-only formatting
  test_support.rs     cfg(test)-only keys, plans, and render helpers
```

Files may be combined if extraction would produce only re-exports. They may be
split further only when a distinct view or reducer concern has its own behavior
and tests.

## Dependency Direction

```text
typed backend values -> app/events -> runtime
                    \-> display <- render
app immutable state ----------------> render
terminal + runtime + app + render ---> tui::run
```

`app` never imports `render`. Shared presentation needed for confirmation/log
messages lives in `display`, which is pure and may be imported by both. Render
modules never import runtime services or call backend operations.

## App Module

`App` remains one struct so selection, jobs, overlays, and scan snapshots keep
their cross-view invariants. Implementation blocks may live in child modules;
shared methods use the narrowest visibility necessary between those children.

The root reducer retains this interface:

```rust
fn update(&mut self, event: UiEvent) -> Vec<Effect>;
```

Input modules mutate state only through reducer-owned methods and return
effects. Worker modules validate job identity/state before applying results.
No view owns target IDs, selected totals, or cleanup authority independently.

The event protocol remains semantically unchanged. Variant relocation is
allowed; converting variants to generic messages or closures is not.

## Runtime Module

The runtime owns all side-effect interpretation. Service traits remain because
production and fake adapters both exist and worker translation/single-flight
behavior is tested at the seam.

`services` owns the traits and real adapters. `workers` owns scan/inventory/
clean execution and typed event emission. `runtime::mod` owns terminal polling,
channels, cancellation registry, effect dispatch, and joining/terminal state.

The runtime must preserve:

- startup scan through an effect;
- staged cumulative scan progress;
- stale job isolation;
- one active clean worker;
- real cancellation tokens without fabricated completion;
- frozen-plan revalidation and digest equality before execution.

## Display And Render

`display` owns pure presentation used outside frame drawing: sanitized path
text, target/action summaries, command preview quoting, compact IDs, and cleanup
progress messages. It never changes stored paths or plan values.

`render` owns ratatui layout and widgets. View modules accept `&App` or narrower
immutable state. Root helpers route areas and overlays; view modules do not
coordinate jobs or selection transitions.

Theme extraction is justified because every view shares semantic colors and
styles. It is not a configurable design system.

## Test Strategy

- App state tests move with input, selection, worker, job, and confirmation
  ownership. Assertions stay on `App::update` outcomes and visible state.
- Runtime tests use fake adapters and assert event/effect translation,
  single-flight, cancellation, and digest rejection.
- Render tests use `TestBackend` and assert observable text/layout for full,
  degraded, narrow, inventory, rules, confirmation, progress, and job states.
- Shared display tests assert control stripping, Windows verbatim-path display,
  argv quoting, width handling, and non-mutation.

Tests may be placed in `tests.rs` under a directory module when they require
private state from several implementation files; they must still test through
the owning interface rather than private forwarding helpers.

## Risks And Controls

- Rust privacy across child modules can tempt broad `pub(crate)`. Prefer
  private parent ownership and `pub(super)` only where sibling implementation
  genuinely collaborates.
- Splitting reducer impl blocks can scatter transitions. Each event family has
  one owner, and `App::update` remains the routing table.
- Moving format helpers can change user text. Move mechanically first and keep
  render/state assertions before any cleanup.
- Moving tests can accidentally weaken them. Compare named behavior coverage,
  not test count alone.

Rollback is the child work commit after a passing full gate.

