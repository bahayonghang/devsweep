# Design Provenance

## Scope

This note records how `devsweep` relates to other public cleanup tools,
especially Mole (GPL-3.0). It is not legal advice.

## Position

`devsweep` is an independent implementation under the MIT License.

Public tools such as Mole informed the problem framing and acceptance
pressure for developer cleanup work: discover high-value cache and build
artifacts, keep irreversible actions explicit, and prefer inspectable plans
over silent mutation. Those are shared product concerns, not shared code.

## What was not copied

The following were not copied or lightly rewritten from Mole or any other
third-party cleanup tool:

- source code
- fixtures and golden files
- rule tables and provider command catalogs
- user-facing copy or UI text
- license text or copyright notices from those projects

Audit remediation work under `.trellis/tasks/` is derived from DevSweep's own
audit report and invariant descriptions, not from another project's
implementation.

## Future review

If any future change introduces content that is derived from a GPL-licensed
work, stop and re-evaluate license obligations before merging. Ordinary
independent reimplementation of ideas and safety requirements does not, by
itself, require changing the MIT baseline chosen for this repository.
