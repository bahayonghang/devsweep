# Implement — Production inspect-advise skill package

## Checklist

1. Copy parent prior-art into
   `skills/devsweep-inspect/reports/prior-art-research.md` after the package
   directory exists. Keep the dated catalog limits.
2. Write `manifest.json` and `agents/interface.yaml` with name
   `devsweep-inspect`.
3. Write `references/safety-contract.md`, `references/cli-playbook.md`, and
   `references/recommendation-report.md`.
4. Write root `SKILL.md` and `README.md`. Keep `SKILL.md` under 14_000 bytes.
5. Write `evals/trigger_cases.json` and `evals/output_cases.json`.
6. Export Skill IR:
   `python C:\Users\lyh\.grok\skills\qiaomu-meta\scripts\export_skill_ir.py skills/devsweep-inspect --output skills/devsweep-inspect/reports/skill-ir.json`
7. Run trigger eval into `reports/trigger-eval.json`.
8. Run `validate_skill.py skills/devsweep-inspect`.
9. Write `reports/creation-handoff.md` with inspected skills, keep/adapt/reject,
   and design/validated/hypothesis labels.
10. Do not edit `AGENTS.md` in this child.

## Validation commands

```powershell
python C:\Users\lyh\.grok\skills\qiaomu-meta\scripts\validate_skill.py skills/devsweep-inspect
python C:\Users\lyh\.grok\skills\qiaomu-meta\scripts\trigger_eval.py skills/devsweep-inspect --cases skills/devsweep-inspect/evals/trigger_cases.json --output skills/devsweep-inspect/reports/trigger-eval.json
```

`just ci` is not required unless Rust files change. They must not change.

## Rollback

Delete `skills/devsweep-inspect/`.
