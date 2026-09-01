---
name: windows-disk-cleanup
description: Use when cleaning up disk space on Windows, analyzing WizTree exports, reclaiming space from caches, temp files, Docker, Python venvs, or dev tool bloat. Use when the user mentions disk full, low space, WizTree, cleanup, or wants to free up storage.
---

# Windows Disk Cleanup with WizTree Analysis

Systematic Windows disk cleanup for developer machines. Two mandatory phases: **Discover** first, then **Execute** only after user approval. All deletions must preserve parent folder timestamps.

## Mandatory Rules

1. **NEVER delete anything without running Discover first.** Always dry-run before execute.
2. **ALWAYS preserve timestamps** on parent folders when deleting subfolders or their contents. See [Timestamp Preservation](#timestamp-preservation).
3. **ALWAYS get explicit user approval** between Discover and Execute phases.

## Phase 1: Discover (Dry-Run)

This phase is **read-only**. No files are deleted. Run this first to understand what can be cleaned.

### Steps

1. **Analyze** -- If user provides a WizTree screenshot, read it and identify top space consumers using the [Space Consumer Reference](#space-consumer-reference) table below.
2. **Copy scripts** -- Copy the bundled scripts from [scripts/](scripts/) into the user's working directory.
3. **Run dry-runs** -- Execute the scripts without `-Execute` to report what would be deleted:
   ```bash
   powershell -ExecutionPolicy Bypass -File ./cleanup.ps1
   powershell -ExecutionPolicy Bypass -File ./clean_venvs.ps1
   powershell -ExecutionPolicy Bypass -File ./clean_venvs.ps1 -Path D:\other\projects
   powershell -ExecutionPolicy Bypass -File ./downloads_report.ps1
   ```
4. **Present findings** -- Show the user a summary table with per-category sizes and estimated total savings.
5. **Ask user** -- Which categories to clean, which to skip. Do NOT proceed to Phase 2 without explicit approval.

### What Discover Reports

| Script | Dry-run output |
|--------|---------------|
| `cleanup.ps1` | Per-category size (temp, Docker, npm, uv, Chrome, Windows junk, Android SDK) + Downloads analysis (top 50 files, top 20 file types, up to 15 duplicate groups). Shows Drive C: free space. Color-coded output with elapsed time. |
| `clean_venvs.ps1` | Every .venv found, its size, whether it has `pyvenv.cfg`, total across all venvs. Color-coded by size (red >2 GB, yellow >1 GB). Per-venv elapsed time and running total. Terminal output only (no report file). |
| `downloads_report.ps1` | Top 50 files, top 25 file types, up to 25 duplicate groups (>1 MB same size with waste calculation), `.venv` summary. Saved to timestamped `.txt` in script directory. Note: only searches for `.venv` folders (not `venv` or numbered variants). |

## Phase 2: Execute (Deletions)

Only run this after the user has reviewed Phase 1 output and explicitly approved.

### Steps

1. **Run with `-Execute`**:
   ```bash
   powershell -ExecutionPolicy Bypass -File ./cleanup.ps1 -Execute
   powershell -ExecutionPolicy Bypass -File ./cleanup.ps1 -Execute -SkipDocker -SkipAndroid
   powershell -ExecutionPolicy Bypass -File ./clean_venvs.ps1 -Execute
   powershell -ExecutionPolicy Bypass -File ./clean_venvs.ps1 -Execute -ToRecycleBin
   ```
2. **Review report** -- `cleanup.ps1` writes `cleanup_report_TIMESTAMP.txt` (in script directory) logging every item with status (DELETED, LOCKED, WOULD DELETE, EXECUTED). The EXECUTED status is used for native CLI commands like `npm cache clean --force` and `uv cache clean`.
3. **Check Drive C:** -- Both `cleanup.ps1` and `clean_venvs.ps1` show Drive C: free space before and after execution.
4. **Verify** -- User runs WizTree again to confirm space reclaimed.
5. **Handle failures** -- If `-ToRecycleBin` fails on deep paths (>260 chars), re-run those specific folders without `-ToRecycleBin` for permanent delete.

## Timestamp Preservation

**MANDATORY for all deletion operations.** When deleting a subfolder (venv, cache, temp), the parent folder's LastWriteTime, LastAccessTime, and CreationTime change. This must be prevented.

### Pattern: Save and Restore

Every script that deletes subfolders MUST save parent timestamps before deletion and restore them after:

```powershell
$parentDir = Split-Path $targetPath -Parent
$parentItem = Get-Item $parentDir -Force -ErrorAction SilentlyContinue
$origLastWrite  = $parentItem.LastWriteTime
$origLastAccess = $parentItem.LastAccessTime
$origCreation   = $parentItem.CreationTime

# --- perform deletion ---
Remove-Item $targetPath -Recurse -Force -ErrorAction Stop

# --- restore timestamps ---
$parentItem = Get-Item $parentDir -Force -ErrorAction SilentlyContinue
if ($parentItem) {
    $parentItem.LastWriteTime  = $origLastWrite
    $parentItem.LastAccessTime = $origLastAccess
    $parentItem.CreationTime   = $origCreation
}
```

### Where This Applies

- `clean_venvs.ps1` -- **Already implemented.** Saves/restores parent timestamps around every venv deletion.
- `cleanup.ps1` -- Operates on system caches (temp, npm, Docker) where parent timestamp preservation is less critical, but the pattern should be applied if extending to user project directories.
- Any new cleanup scripts you generate -- **Always include this pattern.**

### Why This Matters

Developers rely on folder timestamps to know when a project was last modified. Deleting a `.venv` inside `my-project/` should NOT change `my-project/`'s LastWriteTime from "2024-06-15" to "today". Without this, WizTree and file explorers show misleading modification dates.

## Bundled Scripts

Three ready-to-use PowerShell scripts in [scripts/](scripts/). All use `$env:` variables for portability -- no hardcoded personal paths. Reports are written to `$PSScriptRoot` (the script's own directory).

| Script | Purpose | Key Parameters | Default Path |
|--------|---------|---------------|-------------|
| [scripts/cleanup.ps1](scripts/cleanup.ps1) | Main cleanup: temp, Docker, npm/uv caches, Chrome (multi-profile), Windows junk, Android SDK, Downloads report | `-Execute`, `-SkipDocker`, `-SkipAndroid` | Uses `$env:` variables per category |
| [scripts/clean_venvs.ps1](scripts/clean_venvs.ps1) | Python .venv cleanup with pyvenv.cfg safety, timestamp preservation, Recycle Bin option | `-Execute`, `-ToRecycleBin`, `-Path <dir>` | User's home directory |
| [scripts/downloads_report.ps1](scripts/downloads_report.ps1) | Downloads analysis: top 50 files, file types, duplicates, .venv summary | `-Path <dir>` | User's Downloads folder |

### Script Features

- **Native CLI fallback**: `cleanup.ps1` prefers native cache commands (`npm cache clean --force`, `uv cache clean`) when available, falls back to `Remove-Item` if CLI not found.
- **npm cache path fallback**: Checks `$env:LOCALAPPDATA\npm-cache` first, then `$env:APPDATA\npm-cache`.
- **Chrome multi-profile**: Discovers all Chrome profiles (Default + Profile *) and cleans 5 cache subdirectories per profile.
- **Chrome running warning**: Warns user if Chrome is running (cached files may be locked).
- **Progress tracking**: All scripts show elapsed time, per-item progress counters (`X/Y`), and running totals during measurement and deletion.
- **Color-coded output**: Categories color-coded by severity/size; venvs colored red (>2 GB), yellow (>1 GB), white (smaller).

## Space Consumer Reference

| Category | Typical Location | Typical Size | Safe to Delete? |
|----------|-----------------|-------------|-----------------|
| Temp files | `$env:TEMP`, `C:\Windows\Temp` | 5-30 GB | Yes (skip locked) |
| Docker | `AppData\Local\Docker` | 10-50 GB | Yes (if not using) |
| npm cache | `AppData\Local\npm-cache` | 2-10 GB | Yes (rebuilds on demand) |
| uv cache | `AppData\Local\uv\cache` | 2-10 GB | Yes (rebuilds on demand) |
| pip cache | `AppData\Local\pip\cache` | 1-5 GB | Yes (rebuilds on demand) |
| Chrome cache | `AppData\Local\Google\Chrome\...\Cache` | 2-10 GB | Yes (regenerates) |
| Recycle Bin | System | Variable | Yes |
| Windows Update | `C:\Windows\SoftwareDistribution\Download` | 1-5 GB | Yes |
| Thumbnail cache | `AppData\Local\Microsoft\Windows\Explorer` | 0.1-1 GB | Yes (regenerates) |
| Android SDK | `AppData\Local\Android` | 5-15 GB | Ask user |
| Python .venv | Inside project folders | 0.5-3 GB each | Yes (recreate with `pip install`) |
| Ollama models | `~/.ollama` | 5-50 GB | Ask user |
| LM Studio | `~/.lmstudio` | 5-30 GB | Ask user |
| node_modules | Inside project folders | 0.2-2 GB each | Yes (recreate with `npm install`) |

### Leave Alone

- `AppData\Local\Programs` -- installed applications
- `AppData\Local\Microsoft` -- OS/app data
- `ms-playwright` -- if actively used for testing
- WSL distributions -- contains Linux filesystems
- CapCut / other creative apps -- user content

## Python .venv Safety Checks

- **Verify with `pyvenv.cfg`**: Only delete folders containing `pyvenv.cfg` at root. Folders named `venv` without this file are NOT Python venvs.
- **Search patterns** (`clean_venvs.ps1`): `.venv`, `venv`, `.venv310`, `.venv39`, and regex `\.?venv\d+`. Deduplicated by FullName -Unique.
- **Search patterns** (`downloads_report.ps1`): Only searches for `.venv` folders. Does NOT search `venv` or numbered variants.
- **Recycle Bin option**: `-ToRecycleBin` uses Shell API which fails on paths >260 chars. Always offer permanent delete as fallback.

## Quick Reference

| Command | Phase | Purpose |
|---------|-------|---------|
| `.\cleanup.ps1` | Discover | See what would be cleaned |
| `.\cleanup.ps1 -Execute` | Execute | Delete files |
| `.\cleanup.ps1 -Execute -SkipDocker` | Execute | Delete files, skip Docker |
| `.\clean_venvs.ps1` | Discover | Find all .venv folders (default path) |
| `.\clean_venvs.ps1 -Path D:\code` | Discover | Find .venvs in custom path |
| `.\clean_venvs.ps1 -Execute` | Execute | Permanently delete .venvs |
| `.\clean_venvs.ps1 -Execute -ToRecycleBin` | Execute | Send .venvs to Recycle Bin |
| `.\downloads_report.ps1` | Discover | Analyze Downloads folder |
| `.\downloads_report.ps1 -Path D:\data` | Discover | Analyze custom directory |

All scripts run from Git Bash as: `powershell -ExecutionPolicy Bypass -File ./script.ps1 [flags]`

## Critical Lessons Learned

- **PowerShell 5.1 encoding**: Without BOM defaults to ANSI. Never use Unicode chars (box-drawing, em-dashes) in generated scripts. ASCII only.
- **NTFS small-file deletion is slow**: Tens of thousands of small files (uv cache, venvs) take 10-30+ min. Warn users. This is not a hang.
- **Git Bash + PowerShell interop**: `$_` gets mangled by bash. Always use `-File` flag, never `-Command` with complex expressions.
- **Recycle Bin Shell API limit**: `Microsoft.VisualBasic.FileIO` fails on paths >260 chars with "The system call level is not correct". Offer permanent delete as fallback.
- **Recycle Bin caveat for `-ToRecycleBin`**: When `clean_venvs.ps1 -Execute -ToRecycleBin` sends venvs to the Recycle Bin, space is only freed when the user empties it. Note: `cleanup.ps1 -Execute` empties the Recycle Bin automatically via `Clear-RecycleBin -Force`.
- **Docker cleanup order**: `docker system prune -a --volumes -f` first, stop Docker Desktop, then remove `AppData\Local\Docker`.
- **Cache deletion is safe**: npm, uv, Chrome caches all regenerate on demand. Zero functional impact. pip cache is not cleaned by the bundled scripts but is listed in the Space Consumer Reference for manual cleanup if needed.

## Common Mistakes

- Deleting without dry-run first -- always run Discover phase.
- Forgetting to preserve parent timestamps -- always save/restore around deletions.
- Writing Unicode in PowerShell scripts -- always ASCII for PS 5.1.
- Deleting non-venv folders named "venv" -- always check for `pyvenv.cfg`.
- Promising Recycle Bin for deep paths -- Shell API fails on long paths.
- Not showing progress -- users think the script is hung during slow NTFS deletions.
