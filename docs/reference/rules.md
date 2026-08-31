# Rules

Rules is a read-only projection of the shipped cleanup registry. This surface
cannot edit, import, or execute rule code.

Run the inspect-only catalogue:

```powershell
devsweep clean rules list
devsweep clean rules show --id rust.target
```

## Projection

Each row exposes stable identity, source (`shipped_registry`), safety class,
platform applicability, risk, and inspect-only rationale. The projection matches
the authoritative registry used by scanners and plan validation.

Inspect-only and deferred rules remain ineligible for cleanup execution. Always
inspect `clean rules` after upgrading DevSweep, because the catalog is part of
the program version's behavior.

## Project and global coverage

Project-level IDs cover Rust `target/`, Node `node_modules` and common build
caches, Python virtual environments, tool caches, and `__pycache__`. Global
providers include npm, pip, pnpm, Yarn, Go, Gradle, Maven, Ivy, NuGet,
JetBrains, and Hugging Face. Cargo home and several caches stay inspect-only.
Docker remains deferred.

## 规则（简体中文）

规则是内置注册表的只读投影，不能在此编辑、导入或执行。升级后请重新查看 `clean rules`，因为目录属于该程序版本的行为。
