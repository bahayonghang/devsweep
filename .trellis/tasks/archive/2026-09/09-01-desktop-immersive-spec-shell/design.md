# Design - Spec and immersive shell

Follow parent `09-01-desktop-mole-visual-redesign/design.md` sections 1-3
and 6.

Owned files:

- `.trellis/spec/desktop-frontend/{index,component-guidelines}.md`
- `desktop/src/app-shell/AppShell.tsx` and tests
- `desktop/src/styles.css` shell/token rules
- optional additive `shell.v1.more` in `resources/i18n/{en,zh-CN}.json`
  plus `SHELL_V1_KEYS` freeze update

Do not edit Clean/Software/Optimize/Analyze/Status page structure except
where global CSS would otherwise leave a light pane. Mode children own
their first screens.

Sweep body lives in the shell as a reusable CSS class so later children
can place it without a second motif.
