---
layout: home

hero:
  name: DevSweep
  text: Safety-first developer cleanup
  tagline: Inspect disk usage, create an auditable plan, and execute only after explicit review.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: View CLI reference
      link: /reference/cli

features:
  - title: Plan before cleanup
    details: Scans produce versioned reports with target evidence, risk, size completeness, and a typed cleanup intent.
  - title: Fail closed
    details: Plans are validated and actions are reconstructed from DevSweep's built-in rule registry before execution.
  - title: Inspectable by design
    details: Inventory, rule listing, dry runs, and execution audit logs make the cleanup decision visible at every step.
---

## The normal workflow

1. [Scan](/guide/scan) a project, global providers, or both.
2. Review the generated plan, including its health diagnostics and size bounds.
3. Run [`clean` without `--execute`](/guide/clean) to dry-run a saved plan.
4. Add `--execute` only when the saved plan and its selected targets are ready.

DevSweep is deliberately conservative. It does not permanently delete files,
does not clean Docker, and treats Cargo home as inspect-only. Start with the
[safety model](/guide/safety-model) before using an execution command.

## Choose a language

The root site is English. Use the language menu to open the equivalent
[Simplified Chinese documentation](/zh/).
