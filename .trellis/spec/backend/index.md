# Backend Development Guidelines

> Best practices for backend development in this project.

---

## Overview

This directory contains backend guidelines for the Rust CLI, cleanup-plan
domain model, scanner, and future execution/audit code.

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Module organization and file layout | Rust crate conventions |
| [Database Guidelines](./database-guidelines.md) | File persistence and compatibility | JSON plans, audit, and settings |
| [Desktop Preferences](./desktop-preferences.md) | Desktop store, IPC, main/HUD consumers | Closed V1 preference contract |
| [Error Handling](./error-handling.md) | Error types, handling strategies | anyhow CLI/scanner conventions |
| [Quality Guidelines](./quality-guidelines.md) | Code standards, forbidden patterns | Foundation, plan validation, and scanner conventions |
| [Logging Guidelines](./logging-guidelines.md) | Structured logging, log levels | tracing to stderr |

---

## Pre-Development Checklist

Before backend changes:

1. Read [Quality Guidelines](./quality-guidelines.md) for safety-first cleanup
   constraints.
2. Read [Directory Structure](./directory-structure.md) before adding or
   splitting Rust modules.
3. Read [Error Handling](./error-handling.md) before changing CLI, scanner, or
   filesystem error paths.
4. Read [Logging Guidelines](./logging-guidelines.md) before adding diagnostics
   to commands that may also emit JSON.
5. Read [Database Guidelines](./database-guidelines.md) before adding any
   persistent state or file-format contract.

Always run `just ci` before reporting backend work complete.

---

**Language**: All documentation should be written in **English**.
