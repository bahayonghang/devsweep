# Bounded final evidence review

Date: 2026-10-14. Reviewer: `backend_contract_check` (`trellis-check`). This is a read-only source/evidence review with task-document wording fixes, completed **while the final no-debugger idle run is still in progress**. It is separate from the parent's later idle/final acceptance record.

## Findings (fixed)

The parent execution-status F row and task-map introduction used the collective wording “production component benchmarks pass.” Tightened both to distinguish the existing review/preview benchmark's frozen-budget PASS from the new home workload's **MEASURED** result, with retention/DOM bounds verified and no new latency budget. Native/manual and mbx gaps remain explicit. No frozen product, test or measurement code changed.

## Evidence checked

- Final `just-ci-final.log` reports 191 + 12 + 8 + 5 + 480 + 1 + 94 = **791 passing tests**, 0 failed and 5 ignored across the relevant binaries; format/check and final workspace Clippy complete. The MSIX test-only isolation and Node runtime-path repair are separately documented with before/after evidence.
- `desktop-web-check-verified.log` and F gates record **471 Vitest + 5 Node tests**, generated type parity, lint, TypeScript and production build. The packaged main/state/CSS asset identities agree with the final build manifest. This review read the frozen manifests and logs; it did not rehash large artifacts during idle sampling.
- `native-cli-04/summary.json` contains both scopes in English and Chinese, successful digest-bearing previews, equal same-scope digests across locales, unchanged sentinels and `cleanup_executed:false`. It is real CLI evidence; it does not establish the final native desktop's preview flow.
- `native-acceptance.md` keeps the final native desktop preview **UNVERIFIED**. The earlier working desktop's geometry/global IPC observations and matching frontend assets are not combined with the fixed CLI preview to claim final native parity. The old control binary's unavailable listener is recorded without declaring a new-binary regression.
- Existing review/preview records retain the five-warmup/thirty-sample method and frozen fixtures. The 10k interaction p95 values 14.1 / 14.3 / 13.6 ms are within the inherited 100 ms budget. The review DOM increase is disclosed, while full 10k retention and 100 mounted rows are preserved.
- The separate home record reports 14.0 / 14.1 / 14.2 ms, thirty samples per operation, 10k retained owners and ten mounted owner articles. Its synthetic projection, component-only scope, excluded native IPC/aggregation and absence of a new latency budget are explicit.
- Parent AC3–AC5 and AC7–AC10 remain open/partial where required. mbx workspace ownership/capacity/executable qualification, installed uv 0.13 support, native taskbar/tray/installer, native screen-reader/additional font matrix and inherited target-checkbox localization limits are not converted into PASS.

## Findings (not fixed) / pending acceptance

No additional substantive product safety regression or unsupported native PASS claim was found within this bounded final review. Previously documented gaps remain; this is not parent completion.

Final idle is still running. The analyzer's `status` summarizes numeric host gates, while `protocol_coverage_status` explicitly requires review. Before recording formal idle PASS, the parent must inspect all five retained windows' sample counts, first/last coverage, observed spans and maximum gaps against the declared 200 ms protocol, along with exact PID/creation identity, no debugger, normal close, no survivors and sentinel results. This review neither runs nor approves that pending measurement. Unavailable/changing tree identity evidence must remain separate from host gates.

## Verification boundary

No test, build, browser/native operation or benchmark was run for this final review, so the quiet idle measurement was not interrupted. Validation here consists of reading current task/code contracts, saved gate logs, manifests and result artifacts. The parent owns the later source freeze/final acceptance update and any necessary final document-only whitespace check.
