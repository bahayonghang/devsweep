# Frontend Development Guidelines

> Best practices for frontend development in this project.

---

## Overview

This directory contains guidelines for the Rust terminal UI built with
`ratatui`. It does not describe a web frontend.

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Module organization and file layout | ratatui module conventions |
| [Component Guidelines](./component-guidelines.md) | Component patterns, props, composition | render-function conventions |
| [Hook Guidelines](./hook-guidelines.md) | Custom hooks, data fetching patterns | TUI event/update conventions |
| [State Management](./state-management.md) | Local state, global state, server state | app-state boundaries |
| [Quality Guidelines](./quality-guidelines.md) | Code standards, forbidden patterns | TestBackend and purity rules |
| [Type Safety](./type-safety.md) | Type patterns, validation | Rust model sharing |

---

## Pre-Development Checklist

Before TUI/frontend changes:

1. Read [Quality Guidelines](./quality-guidelines.md) for render purity and
   `TestBackend` expectations.
2. Read [Directory Structure](./directory-structure.md) before splitting
   `src/tui.rs` into submodules.
3. Read [Component Guidelines](./component-guidelines.md) before adding new
   render functions or widgets.
4. Read [State Management](./state-management.md) and
   [Hook Guidelines](./hook-guidelines.md) before adding event/update/worker
   behavior.
5. Read [Type Safety](./type-safety.md) before adding view state or plan-file
   decoding.

Always run `just ci` before reporting TUI work complete.

---

**Language**: All documentation should be written in **English**.
