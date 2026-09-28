# Primary References

Accessed: 2026-09-26. References support technical feasibility and palette provenance. Project behavior is established by repository-findings.md.

1. Catppuccin palette: https://catppuccin.com/palette/ . Source for Latte and Mocha color values. Import only the required palette data, record upstream attribution/license, and keep DevSweep semantic token mapping local.
2. Catppuccin palette source: https://github.com/catppuccin/palette . Pin the palette revision/license in the implementation evidence before copying values. Do not introduce a palette runtime package just to obtain constants.
3. Base UI Select: https://base-ui.com/react/components/select . Provides unstyled Select parts, Portal/Positioner, selected indicators, and positioning controls. The documentation recommends Combobox when a list needs filtering.
4. Base UI Combobox: https://base-ui.com/react/components/combobox . Provides input, list, item, empty-state, portal, and positioning parts. The exact approved release, React 19 peer compatibility, virtualized-list behavior, license, and lockfile are implementation checks; no version is selected in this planning turn.
5. WAI-ARIA Combobox pattern: https://www.w3.org/WAI/ARIA/apg/patterns/combobox/ . Defines accessible name/value, keyboard use, listbox focus, and dismissal behavior. A menu button does not convey a selected form value by itself.
6. Microsoft IDWriteFactory3::GetSystemFontCollection: https://learn.microsoft.com/en-us/windows/win32/api/dwrite_3/nf-dwrite_3-idwritefactory3-getsystemfontcollection . includeDownloadableFonts controls cloud inclusion. checkForUpdates requests an immediate catalogue refresh. The documented client baseline is Windows 10.
7. Microsoft custom font sets: https://learn.microsoft.com/en-us/windows/win32/directwrite/custom-font-sets-win10 . Describes installed system collections separately from custom/application/cloud font sets. This task uses the installed local collection only.

## Decision from references

Implementation source pinned on 2026-09-26: catppuccin/palette revision 07d02aa110ef9eb7e7427afca5c73ba9cf7f8ebd. The task research directory contains catppuccin-pinned.json (Latte/Mocha data only) and catppuccin-LICENSE (MIT, Copyright 2021 Catppuccin), read through the GitHub API at that exact revision. The palette child must include source/license attribution with the distributed constants.

Use a mature unstyled selection primitive rather than writing a new keyboard/focus engine. Use native DirectWrite enumeration rather than treating document.fonts or a short list of CSS guesses as an installed-font catalogue. Both production dependencies were explicitly approved on 2026-09-26; see approval.md. No external service receives font names.

Codex and Claude presets in this plan are original, inspired proposals. No source claim establishes exact official Codex or Claude UI colors.
