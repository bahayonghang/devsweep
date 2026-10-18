# C10 Windows high-contrast window sample (round 3)

Date: 2026-10-18. Binary: `devsweep-gpui-spike.exe`, SHA-256
`DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34`, 14179328
bytes (built through `just gpui-build`, so the link is `/Brepro`-deterministic).

## Method

The app reads the real system setting through `SPI_GETHIGHCONTRAST`
(`view.rs:18-21`), so this sample turns the genuine Windows high-contrast flag on
for the duration of one launch and then restores it. Nothing was faked inside the
app and no application code was changed.

| Reading | Value |
| --- | --- |
| `SPI_GETHIGHCONTRAST` before | `dwFlags = 0x0000007E`, `HCF_HIGHCONTRASTON` off |
| `SPI_SETHIGHCONTRAST` on | returned `True`, `dwFlags = 0x0000007F` |
| `SPI_GETHIGHCONTRAST` during | `HCF_HIGHCONTRASTON` **on** |
| Window | 1318 x 947, appeared in 1736 ms, `PrintWindow` returned `True` |
| Shutdown | `WM_CLOSE`, exit code 0 |
| `SPI_SETHIGHCONTRAST` restore | returned `True` |
| `SPI_GETHIGHCONTRAST` after | `dwFlags = 0x0000007E`, `HCF_HIGHCONTRASTON` off |
| Restored to original | **yes** |

## What the window shows

Screenshot: `c10-highcontrast-window.png` (27057 bytes, sha256
`47f9045c83338e3642419038ec81991b88ce1fc0a04eab96a9a1983a43558275`).

Sampled colours over a 7-pixel grid:

```
000000 x22352    A6A6A6 x1570    FFFFFF x1524    717171 x112    AAAAAA x8
```

- Canvas is pure black (`000000`), matching `HIGH_CONTRAST_CANVAS`.
- Text and the selected pill are pure white (`FFFFFF`), matching
  `HIGH_CONTRAST_TEXT` and `HIGH_CONTRAST_PILL`.
- The selected `清理` tab renders as a white pill with black text; the four
  unselected tabs (`软件`, `优化`, `分析`, `状态`) render as white text on the
  black capsule, and the capsule carries a white border.
- The beaver icon, the `清理` heading and the line
  `先审查每个清理目标，再移动任何内容。` all remain legible.
- The note field is a white box with a black caret and the zh-CN placeholder
  `在此输入`, so the input affordance survives the palette switch.

The rendered result matches the constants in `view.rs:53-59` and the
`high_contrast_palette_is_distinct` unit test, but this is the actual native
window rather than a unit assertion.

## Scope

This closes only the "actual Windows high-contrast window sample" clause of
C10-AC3. It is **not** a Narrator session, and it does not measure reduced-motion
frame intervals. Those stay NOT_RUN.

The system setting was restored in the same run and verified afterwards. No
other system parameter was modified.
