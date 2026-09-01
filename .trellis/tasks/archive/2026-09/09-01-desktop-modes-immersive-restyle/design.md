# Design - Remaining mode restyle

Follow parent `design.md` section 5.

Owned directories: `desktop/src/modes/{software,optimize,analyze,status}/`
and their tests/CSS. Shared canvas tokens come from spec-shell. Do not
add IPC fixtures or generated types unless a decoder test fails because
of CSS-only class changes (it should not).
