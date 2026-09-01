# Protection

Protection is a persistent exact-path allowlist. Matching cleanup targets are
refused. It cannot expand cleanup scope.

## Storage

The store is `%APPDATA%\devsweep\protected-paths.json` (Windows). A sidecar
lock covers load → version/byte validation → canonical exact-path compare →
mutate → same-directory temp write/flush → atomic replace. Each writer reloads
after acquiring the lock, so two CLI/TUI/Desktop processes cannot lose an
update.

Invalid UTF-8/JSON and unknown or newer versions preserve the original bytes
and return `protection_store_unavailable`. DevSweep never treats those files as
an empty list, auto-clears them, or deletes them.

## Mutations

`clean protect add|remove --path <PATH> --confirm` (CLI) and the matching
TUI/Desktop confirmations are required. Each mutation emits `requested`,
`committed`, or `failed` through the Clean V1 audit writer. The payload has
action, operation id, stable result code, and SHA-256 of the canonical
identity. It never stores the raw path.

Pre-mutation audit failure blocks the store change. Post-replace audit failure
returns fail-closed/unknown without rolling the store back or repeating the
mutation.

## 保护（简体中文）

保护列表是精确路径白名单，匹配的清理目标会被拒绝，且不能扩大清理范围。损坏或未知版本的存储会原样保留字节并返回 `protection_store_unavailable`。变更必须确认，审计只记录规范身份的 SHA-256，不记录原始路径。
