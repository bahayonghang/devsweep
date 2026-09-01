---
name: storage-cleanup
description: >
  Scan the system for wasted disk space and clean it up. Covers AI coding tools
  (Claude Code/Desktop, Codex, Gemini CLI, Cursor, Trae, Antigravity, Windsurf,
  Codeium, Ollama, DiffusionBee), package manager caches (npm, yarn, pip, pnpm,
  uv, Homebrew, CocoaPods, Maven, Gradle), dev tool runtimes (Docker, Xcode,
  Playwright/Puppeteer/Cypress, nvm Node versions), and macOS app containers.
  Use this skill whenever the user mentions disk space, storage cleanup, freeing
  space, "what's eating my disk", cache cleanup, or wants to reclaim storage.
  Also triggers on Chinese equivalents like "清理存储", "磁盘清理", "磁盘空间",
  "存储空间不够". Even if the user only asks about ONE tool's storage (e.g.
  "how big is my npm cache"), use this skill because it provides the full picture
  that helps the user make informed decisions.
---

# Storage Cleanup

A systematic disk space auditor and cleaner for macOS development machines.

Modern dev machines accumulate enormous hidden storage from AI coding tools,
package managers, and development runtimes. Each tool caches aggressively but
never auto-cleans. This skill finds it all and helps the user reclaim space
safely.

## Workflow

### Phase 1: Snapshot & Scan

Take a "before" snapshot, then scan all known storage locations in parallel.

```bash
# Before snapshot
df -h / | tail -1
```

Run ALL of these scan commands in parallel (use multiple Bash tool calls in one message):

**System overview:**
```bash
du -sh ~/.[!.]* 2>/dev/null | sort -rh | head -30
```

**AI/Coding tools:**
```bash
# Each as a separate parallel call for speed
du -sh ~/.claude ~/.codex ~/.gemini ~/.cursor ~/.trae ~/.antigravity ~/.windsurf ~/.codeium ~/.ollama ~/.diffusionbee 2>/dev/null
du -sh ~/Library/Application\ Support/Claude ~/Library/Application\ Support/Cursor ~/Library/Application\ Support/Antigravity ~/Library/Application\ Support/Trae 2>/dev/null
du -sh ~/Library/Application\ Support/com.openai.atlas ~/Library/Caches/com.openai.atlas 2>/dev/null
```

**Package manager caches:**
```bash
du -sh ~/.npm/_cacache ~/.npm/_npx 2>/dev/null
du -sh ~/.cache/uv ~/.cache/huggingface ~/.cache/puppeteer ~/.cache/modelscope 2>/dev/null
du -sh ~/Library/Caches/Yarn ~/Library/Caches/pip ~/Library/Caches/pnpm ~/Library/Caches/CocoaPods ~/Library/Caches/Homebrew ~/Library/Caches/pypoetry ~/Library/Caches/node-gyp 2>/dev/null
du -sh ~/.m2/repository ~/.gradle/caches ~/.cocoapods/repos 2>/dev/null
```

**Dev tool runtimes:**
```bash
du -sh ~/Library/Developer/Xcode/DerivedData ~/Library/Developer/CoreSimulator/Devices 2>/dev/null
du -sh ~/Library/Caches/ms-playwright ~/Library/Caches/ms-playwright-go ~/Library/Caches/Cypress 2>/dev/null
ls -d ~/.nvm/versions/node/* 2>/dev/null | while read d; do du -sh "$d"; done | sort -rh
```

**App containers (often the biggest surprise):**
```bash
du -sh ~/Library/Containers/*/ 2>/dev/null | sort -rh | head -15
```

**Library top-level:**
```bash
du -sh ~/Library/Caches ~/Library/Application\ Support ~/Library/Containers ~/Library/Developer 2>/dev/null
```

For any tool that shows >500 MB, drill into its subdirectories to understand the breakdown:
```bash
du -sh <path>/*/ 2>/dev/null | sort -rh | head -10
```

### Phase 2: Report

Present findings as a categorized table. Group items by type and annotate each
with a safety level:

| Safety | Meaning | Examples |
|--------|---------|----------|
| SAFE | Pure cache, will regenerate on demand | npm _cacache, pip cache, Yarn cache, Xcode DerivedData, brew cache |
| SAFE | Old sessions/logs with no functional impact | Codex archived_sessions, Claude debug/telemetry, tool logs |
| ASK | Functional data the user might still want | Ollama models, Docker images, old Node versions |
| ASK | App data that can only be cleaned in-app | WeChat media, Chrome browsing data |
| KEEP | Active configuration and project state | ~/.claude/projects, ~/.claude/skills, tool settings |

Format the report like this:

```
## Disk Space Audit

**Current free space: X GB / Y GB total**

### AI/Coding Tools — XX GB total
| Item | Size | Safety | Notes |
|------|------|--------|-------|
| ... | ... | SAFE/ASK | ... |

### Package Manager Caches — XX GB total
| ... |

### Dev Tool Runtimes — XX GB total
| ... |

### App Containers — XX GB total
| ... |

### Total reclaimable: ~XX GB (SAFE) + ~XX GB (ASK)
```

### Phase 3: User Decision

After presenting the report, ask the user what to clean. Offer three presets
plus custom selection:

1. **Conservative** — SAFE items only (caches, logs, old sessions)
2. **Aggressive** — SAFE + all ASK items
3. **Custom** — "Tell me which items to include/exclude"

Also ask about specific ASK items individually:
- "Do you still use Ollama? (5.8 GB in models)"
- "Do you still play 金铲铲? (6.8 GB)"
- etc.

### Phase 4: Execute Cleanup

Run cleanup commands in parallel where independent. Group by category.

**Package manager caches** (safe, run these via their official CLI when possible):
```bash
npm cache clean --force
yarn cache clean
pip cache purge
pnpm store prune
uv cache clean
brew cleanup -s
pod cache clean --all
```

**AI tool sessions** (delete old data):
```bash
rm -rf ~/.codex/sessions ~/.codex/archived_sessions
rm -rf ~/.claude/debug ~/.claude/telemetry ~/.claude/shell-snapshots
# etc. based on user selection
```

**Dev runtimes:**
```bash
rm -rf ~/Library/Developer/Xcode/DerivedData
# For Node versions, delete by path: rm -rf ~/.nvm/versions/node/vX.Y.Z
# For simulators: xcrun simctl delete unavailable (or rm -rf if xcrun unavailable)
```

For each group, echo a confirmation after completion so the user sees progress.

**Permission errors**: Some files (especially Cursor backups with
`com.apple.provenance` xattr) resist deletion. If `rm -rf` fails, try:
1. `chmod -R u+w <path>` then retry
2. `xattr -r -d com.apple.provenance <path>` then retry
3. If still failing, report the residual size and suggest `sudo rm -rf` as a
   manual step

### Phase 5: After Report

Show the before/after comparison:

```
## Cleanup Complete

| Metric | Before | After | Freed |
|--------|--------|-------|-------|
| Free space | 6.4 GB | 148 GB | 142 GB |

### What was cleaned:
- npm cache: 22.5 GB
- Codex sessions: 11 GB
- ...

### Remaining large items (for reference):
- WeChat: 36 GB (clean in-app)
- ...
```

## Important Notes

- **Never delete without asking.** The scan phase is read-only. All destructive
  actions happen only after the user explicitly chooses what to clean.
- **Explain WHY things are big.** Users are often surprised by the sizes. A
  brief explanation ("Codex saves full conversation transcripts including all
  code and outputs for every session — that's why 50 sessions can reach 9 GB")
  helps them make better decisions and understand the cleanup value.
- **macOS-specific paths.** This skill assumes macOS with `~/Library/`. The
  concepts apply to Linux too but paths differ — adapt if `uname` returns Linux.
- **Docker needs its daemon.** `docker system prune` only works if Docker is
  running. If `docker info` fails, tell the user to start Docker Desktop first
  or suggest deleting `~/Library/Containers/com.docker.docker/` directly if
  they want to remove Docker entirely.
- **Node version management.** Always check `node -v` and `nvm alias default`
  before suggesting which versions to remove. Keep the default + current.
- **Don't touch active configs.** Never delete settings files, project configs,
  or memory directories (`~/.claude/projects/`, `~/.claude/skills/`, etc.)
