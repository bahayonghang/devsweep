# Windows cache boundaries for Wave C

Verified against primary sources on 2026-10-13. These are implementation inputs,
not permission to run the referenced reset/deletion instructions. Test only
disposable fixtures. The approved coverage matrix remains the acceptance contract.

## Browser profiles

- [Chromium user-data documentation](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/user_data_dir.md)
  identifies Chrome's default Windows root as
  `%LOCALAPPDATA%/Google/Chrome/User Data`. Profiles are children of that root;
  root/profile directories contain persistent history, bookmarks and cookies.
  A command-line override can move the root. A default-root-only resolver must
  state that limitation rather than claiming coverage of every installation.
- [Chromium constants](https://raw.githubusercontent.com/chromium/chromium/main/chrome/common/chrome_constants.h)
  distinguish `Default`, `Profile `, `Cache`, `Cookies`, `Local State`,
  `Preferences`, `Network`, and offline/web-application storage. Restrict browser
  candidates to individually allowlisted cache children of identified profiles.
- [Microsoft's Edge path documentation](https://learn.microsoft.com/en-us/deployedge/edge-learnmore-create-user-directory-vars)
  identifies `%LOCALAPPDATA%/Microsoft/Edge/User Data/Default` as a default profile.
  [UserDataDir policy](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-policies/userdatadir)
  can override command-line selection. Unresolved custom/policy layouts cannot
  gain cleanup authority from a similarly named directory.
- [Firefox profile documentation](https://firefox-source-docs.mozilla.org/toolkit/profile/index.html)
  separates persistent `%APPDATA%/Mozilla/Firefox/Profiles` from default local
  `%LOCALAPPDATA%/Mozilla/Firefox/Profiles` caches, with matching leaf names.
  `profiles.ini` records profile roots, including custom absolute roots. Custom
  profiles can colocate persistent and cache data; only the exact cache subtree
  may be selected. New profile groups are not permission to clean group stores.
- [Firefox HTTP-cache documentation](https://firefox-source-docs.mozilla.org/networking/cache2/doc.html)
  identifies cache payloads at `cache2/entries` and the cache index/journal at
  `cache2/index`, `index.log`, and `index.tmp`. Never select a Firefox profile root.

## Editors

- [VS Code user-data resolver](https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/platform/environment/node/userDataPath.ts)
  documents precedence: portable mode, `VSCODE_APPDATA`, `--user-data-dir`, then
  the default app-data location. Windows defaults use `APPDATA` and the product
  name. Do not infer a portable/custom root from an arbitrary caller path.
- [VS Code startup](https://raw.githubusercontent.com/microsoft/vscode/main/src/mainImpl.ts)
  sets Electron `userData` to that resolved root and derives V8 `CachedData` under
  it, partitioned by product commit. This supports a narrowly identified cache
  child, never `User`, `Backups`, extensions, or state databases.
- [Electron app paths](https://github.com/electron/electron/blob/main/docs/api/app.md)
  distinguish application data from session data and identify Chromium `Cache`
  and `GPUCache` subdirectories. Session data also includes cookies and persistent
  local storage; the session root is not a cleanup target.
- [Electron session API](https://www.electronjs.org/docs/latest/api/session)
  documents `Code Cache` as the default generated JavaScript cache directory.
  This does not authorize service-worker caches or offline application stores.
- [Cursor endpoint documentation](https://prod.cursor.com/docs/enterprise/endpoint-security)
  identifies `%APPDATA%/Cursor` as user data, settings and workspace storage, and
  documents standard user/system `Cursor.exe` installation paths. Applying the
  exact Electron cache-child allowlist to a positively identified Cursor root is
  an implementation inference; unknown layouts remain unavailable. The older
  troubleshooting page's whole-app reset is outside this task's scope.

## Implementation constraints

Use a closed family/child list, bounded profile enumeration, no reparse traversal,
and existing protected-path checks. Match profile metadata and exact recognized
application context before granting a trash intent. Browser/editor candidates
start unselected. Process enumeration failure or uncertain owner activity must
yield an explicit unavailable reason; absence of a process is evidence only when
the enumeration succeeded. Recheck activity during preview and execution.

Temporary data has a different policy: only validated current-user temp roots,
individual entries at least seven days old, and no whole-root action. Check the
contents' age/eligibility, not merely parent mtime. Recent, locked, linked,
cloud/protected, or unknown-owner entries remain unavailable. Reuse existing
native filesystem/process helpers instead of introducing a generic plugin API.

Required sentinels include browser credentials/history/offline stores, editor
workspace/global state and agent histories, recent nested temp contents,
same-named foreign directories, and live owner/drift between preview and execute.
