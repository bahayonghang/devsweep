# Design Provenance

## Scope

This note records how DevSweep relates to other public cleanup tools,
especially Mole (GPL-3.0). It is not legal advice.

## Position

DevSweep is an independent implementation under the MIT License.

Public tools such as Mole informed the product problem: identify high-value
developer caches and build artifacts, make irreversible work explicit, and
prefer inspectable plans to silent mutation. Those are shared concerns, not
shared implementation material.

## What was not copied

DevSweep did not copy or lightly rewrite any of the following from Mole or
another third-party cleanup tool:

- source code;
- fixtures or golden files;
- rule tables or provider command catalogs;
- user-facing copy or UI text;
- license text or copyright notices.

Audit-remediation work under `.trellis/tasks/` is derived from DevSweep's own
audit reports and invariant descriptions.

## Future review

If a future change introduces material derived from a GPL-licensed work, stop
and reevaluate the licensing obligations before merging. Independent
reimplementation of ideas and safety requirements does not by itself change the
MIT license baseline.
