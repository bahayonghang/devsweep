# Combined ACL GUI acceptance, run 02

Status: prepared, not executed. The user authorization from the prior session
remains: “允许访问后继续界面验收”. This session did not ask that question again.

## Tool blockage

The official Computer Use skill, its guidance, API, and confirmation policy
were read from
`C:\Users\lyh\.codex\plugins\cache\openai-bundled\computer-use\26.1007.21434`.
This Cursor session has no `node_repl` tool. Importing bundled `@oai/sky`
0.7.6 with the cua_node runtime
`C:\Users\lyh\AppData\Local\OpenAI\Codex\runtimes\cua_node\cfb32733c877621e\bin\node.exe`
resolved the package, then `list_windows` failed before any window list:

`failed to launch codex app-server: program not found`

The package raises app approval only through
`globalThis.nodeRepl.config.createElicitation`, which the Codex-hosted
JavaScript session supplies. Outside that session the approval prompt cannot
be shown. No `codex-computer-use.exe` was spawned, no custom helper client was
written, and no separate UI Automation was used.

The fresh 300-second fixture was not started. There is no GUI observation,
no Scan click, and no CLI checker result for run 02. Run 01 remains the
earlier lifecycle-only record.

## Prepared scripts

Copies of the run 01 scripts, reviewed and not launched:

- `dist/performance/cleanup-earth/run-combined-acl-uia-02.ps1`
- `dist/performance/cleanup-earth/hold-combined-acl-uia-02.py`
- `dist/performance/cleanup-earth/check-combined-acl-cli-02.py`

They differ from run 01 only by the fresh output directory
`dist/native-combined-acl-c3-uia-02` and by the launcher calling the run 02
hold script. They still refuse an existing directory, use the frozen C3
desktop `7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6`
and C3 CLI `3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a`,
keep the isolated environment allowlist, System32-only `PATH`, installed
WebView runtime, no `CARGO_TARGET_DIR`, and no CDP. The denied fixture root
receives a temporary ListDirectory deny, and the launch token must observe
access denied before the hold. The CLI checker still expects exit 5, partial
coverage, and exactly the healthy `node_modules` plus Maven targets while the
GUI is alive, the DACL is unrestored, and `finish.flag` is absent.
