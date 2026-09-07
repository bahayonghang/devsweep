---
layout: home

hero:
  name: DevSweep
  text: Safety-first developer cleanup
  tagline: Inspect disk usage, save an observation, create an auditable plan, and execute only after a live preview digest and --confirm.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: View CLI reference
      link: /reference/cli

features:
  - title: Plan before cleanup
    details: Scans produce versioned observations with target evidence, risk, size completeness, and a typed cleanup intent.
  - title: Fail closed
    details: A new saved plan is validated and actions are reconstructed from DevSweep's built-in rule registry before execution.
  - title: Inspectable by design
    details: Preview digests, nested protection and rules commands, and fixed V1 audit stores make the cleanup decision visible at every step.
---

## The normal workflow

1. [Scan](/guide/scan) a project, global providers, or both, and **save the observation**.
2. Select exact target IDs and save a **new** plan. Observation JSON is not a runnable plan.
3. Run [`clean preview`](/guide/clean) for a live digest. Preview is the dry-run.
4. Execute only with that live digest and `--confirm`.

DevSweep is deliberately conservative. It does not permanently delete files,
does not clean Docker, and treats Cargo home as inspect-only. Start with the
[safety model](/guide/safety-model) before using an execution command.

## Choose a language

The root site is English. Use the language menu to open the equivalent
[Simplified Chinese documentation](/zh/).
