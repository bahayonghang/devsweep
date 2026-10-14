# Provider authority validation

The functional implementation uses the same live binding in CLI and Tauri core services. This evidence records supported capability policy, not a request to clean this machine. All native fixtures have private PATH/home/config directories and only write an invocation record. No fixture invokes an actual package-manager cleanup.

## Reviewed capabilities

| Provider / accepted stable versions | Read-only footprint | Reconstructed owner action | Primary evidence |
| --- | --- | --- | --- |
| npm 6–11 | `npm config get cache` | `npm cache clean --force` | [v6 reference](https://docs.npmjs.com/cli/v6/commands/npm-cache/), [v11 reference](https://docs.npmjs.com/cli/v11/commands/npm-cache/) |
| pnpm 8–10 | `pnpm store path` | `pnpm store prune` | [10.x reference](https://pnpm.io/10.x/cli/store), [maintainer's v8.9 behavior report](https://github.com/pnpm/pnpm/issues/7192). Clone import mode may cause more cached packages to be pruned; retained project files and eventual prune yield must not be inferred from store occupancy. |
| pip 20.1–26.x | `python -m pip cache dir` | `python -m pip cache purge` | [cache reference](https://pip.pypa.io/en/stable/cli/pip_cache/), [caching history](https://pip.pypa.io/en/latest/topics/caching/) |
| Yarn classic 1; modern 2–4 | `cache dir`; modern `config get globalFolder` plus exact `cache` child | `cache clean`; modern `cache clean --mirror` | [classic cache reference](https://classic.yarnpkg.com/lang/en/docs/cli/cache/), [modern cache reference](https://yarnpkg.com/cli/cache/clean) |
| Go 1.15–1.27 | `go env GOMODCACHE`; separate `go env GOCACHE` | `go clean -modcache`; separate `go clean -cache` | [Go command reference](https://pkg.go.dev/cmd/go) displayed go1.27.2, published 2026-10-08; [module guidance](https://go.dev/wiki/Modules) documents GOMODCACHE introduction in Go 1.15 |
| uv 0.4–0.11 | `uv cache dir` | `uv cache clean` | [cache reference](https://docs.astral.sh/uv/concepts/cache/), [official 0.11 changelog](https://github.com/astral-sh/uv/blob/main/changelogs/0.11.x.md) |
| Bun 1.0.3–1.3.x | `bun pm cache` | `bun pm cache rm` | [pm reference](https://bun.sh/docs/pm/cli/pm), [global cache](https://bun.sh/docs/pm/global-cache), [1.0.3 fix](https://bun.sh/blog/release-notes/bun-v1.0.3), [1.3 release](https://bun.sh/blog/bun-v1.3). Versions before 1.0.3 are refused because the owner command did not reliably clear its cache. |

These finite ranges intentionally do not assert future capability compatibility. All numeric version components are validated; prerelease/build suffixes, malformed components and extra tokens are refused. pip and Go documented two-component stable versions are accepted. uv's optional bounded hash/date build annotation is parsed. Unsupported versions remain visible in coverage and cannot reach preview authority.

Windows Node dispatch supports only the documented package entry layout next to an npm/pnpm/Yarn `.cmd`/`.bat`: `node_modules/npm/bin/npm-cli.js`, `node_modules/pnpm/bin/pnpm.cjs`, or `node_modules/yarn/bin/yarn.js`. Native `node.exe` is adjacent or resolved independently from PATH. The shim, script and native executable contents are bound; shell text is never executed. Unknown wrappers and Corepack layouts are explicit unsupported cases. Native providers bind their executable, and pip additionally binds its resolved Python interpreter.

## Nonsecret configuration binding

Only allowlisted cache settings and relevant path environment values are hashed, never entire credential-bearing configuration files. Sources include npm user/explicit global rc files, Windows and Unix pip user/global/virtualenv files and explicit override, Yarn user rc files, documented Go GOENV locations, uv user/system/explicit files, and Bun's `[install.cache]` section. The live footprint must still equal the observed footprint. A changed same-path setting invalidates the confirmation even when a fixture intentionally returns the same footprint.

## Recorded tests

`cargo test --locked --target-dir dist -p devsweep-cli --test provider_authority` passed all three tests on native Windows (8.31 s); raw output: `dist/native-test-isolation/provider-authority-current.txt`.

- Nine positive variants: npm, pnpm, Yarn classic/modern, pip, Go module/build, uv and Bun. Each crosses real CLI scan, saved observation, saved v2 plan, production registry, live preview digest and simulated command execution.
- Four same-path authority replacements: shim, fixed CLI script, native Node and npm configuration. Each refuses before dispatch.
- Seven default configuration-source replacements, including Windows pip/Go/uv files, refuse stale confirmation. An npm credential-only update leaves the digest usable; the token is not bound.
- Unicode/space paths, cache paths containing `warning`, and retained cache sentinels are part of the fixtures.

The fixtures prove DevSweep's closed contracts and dispatch behavior; they do not claim every supported historical package-manager binary was installed or executed. Full canonical quality gate and final reference-B capture are tracked separately.
