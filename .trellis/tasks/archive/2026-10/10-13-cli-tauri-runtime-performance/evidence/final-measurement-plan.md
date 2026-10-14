# Final CLI and Earth measurement schedule

Status: prepared, not executed. The final candidate is a required binary path,
label, binary SHA-256 and source SHA-256 parameter. C3-final remains immutable;
this schedule does not assume that a later C4 has passed its gate.

## Identities and inputs

The static preparation rechecked these executable bytes and the fixture manifest:

| Input | SHA-256 |
| --- | --- |
| Original A CLI | `b7280f91304ec965c025e9d110967896d8b4f1d068603fff792d2ed2eb0c8fd6` |
| Corrected B3 CLI | `1cf858d35b1be1cddaaae6284b605e89b6055024d7162121c09531b5fdc91266` |
| Original A native | `28167d0788adc2d9c18e2b256bcb7ac661f66972896acfc8f79a19cf33ff5f47` |
| Frozen fixture manifest file | `9a3ced86876b50fea1c68727c5ad61a2bef4d7518cb05fde1dfe6a54ff2d40e1` |
| Manifest's 7205-file content identity | `4067adeef4c76e4d6abc7bf8b3c09142b7219f315d8d9c199a9b393836898212` |

The manifest records seed 0 and mtime epoch 946684800. The timed CLI runner
rehashes the actual fixture tree before and after execution; the static review
only reads the existing manifest. Every run refuses a mismatched candidate
binary and an inherited `CARGO_TARGET_DIR`. No source, binary, asset or fixture
is rebuilt or regenerated during measurement. OS file-cache coldness remains
uncontrolled. Existing host/toolchain records are retained; the final slot must
record current power/toolchain metadata before timing, and native CDP records
the actual WebView version, viewport and DPR.

## Counts and order

| Series | Exact operations | Planned duration |
| --- | --- | --- |
| Repaired-provider golden parity | 6 cases × B3/final = 12 CLI scans | Expected under 15 s from the earlier protocol; 20 s per-command observation cap, no forced success |
| Healthy Clean timings | 5 workloads × 14 scans × 2 reference series = 140 CLI scans | Not yet measured; see estimate below |
| Earth presentation resources | 5 A/final pairs = 10 sequential app launches, 20 state windows | Exactly 400 s state settle/retention, approximately 8–10 min including startup, CDP and normal closure |

The 152 CLI scans are command counts, not a claim of 152 total processes:
allowlisted provider children and native WebView helpers are separately owned
and checked for natural settlement.

Each Clean workload/reference combination has separate first-use reference/C
and discarded warmup reference/C runs, followed by five retained pairs in
`reference,C; C,reference; reference,C; C,reference; reference,C` order.
Candidate observations are not shared between the A and B3 series. The actual
order and position accompany every record. The five workloads are `many-100`,
`overlap`, `wide`, `deep`, and `Unicode path 测试`; overlap adds the first nested
project to the same many-100 root. Expected final target counts are 100, 100,
1, 1 and 1 respectively.

There are 56 many-100/overlap scans and 84 shorter scans. The retained historical
A many-100 first-use observation was 48.734 s. Applying that one observation
to all 56 long scans would be about 45.5 min, not a forecast of the final builds.
The first B3/final many-100 pair is part of the declared first-use series and
will refine the estimate without an additional warmup or discarded trial.
The existing top-level and two-level raw summary inventory contained no B3 or
C3 CLI many-100/overlap retained timing series. Native scan/page lifetime spans
are not substitutes for CLI command durations. Reserve roughly an hour for the
CLI stage until the actual first-use pair refines that scheduling estimate.
Every command appends and closes `progress.jsonl`, every completed pair writes
its own JSON record, and every completed workload writes its own summary.
Console output flushes after each command, so the coordinator can report
incremental observations without waiting for the entire batch.

Earth alternates A/C and C/A by pair, and alternates still/animated state order
by pair. Each app has 5 s settling plus 15 s retained for each of the two states.
Only one measured app and its sampler exist at a time. Both builds receive the
same CDP/media/RAF observer; animation requires observed callback progression.

## Comparisons and gates

- Provider cases retain Go native queries, configured roots, npm configured
  query dispatch, nearest-marker nested Python, mixed healthy/missing roots,
  and refused Yarn delegation. B3/final compare the complete V1 envelope and
  exit code without normalization. Expected partial exit 5 remains part of the
  partial case; expected rules, paths, authority, coverage and sentinels are
  checked separately. Provider command logs remain allowlisted and no cleanup
  simulation may run.
- B3/final timed Clean compares the entire V1 JSON envelope and exit 0, including
  target facts/order, intent/selection, size confidence, diagnostics and coverage.
  Healthy A/final comparison omits only the candidate's new
  `data.health.coverage` field, which must be absent in A. Every common field
  and exit remain exact. No broader timestamp, path or authority normalization
  is allowed.
- Five retained timings use median and nearest-rank p95 (the largest of five).
  The inherited guards stay candidate median ≤ reference × 1.20, root peak
  private bytes ≤ reference + 64 MiB, and root peak threads ≤ reference + 2.
  A guard is unavailable if either build has a run with fewer than two valid
  200 ms root samples; it is never reported as zero or pass. Tree resources are
  separate observations. First-discovery and internal phase timing are
  unavailable in these uninstrumented release CLI envelopes.
- Earth records host and owned-tree CPU separately with `100% = one logical
  core`, plus private bytes, threads, handles and process counts. A changed
  process identity set makes that interval's tree CPU unavailable. The five
  state observations yield median and nearest-rank p95; there is no invented
  absolute tree/GPU gate or claim that whole-app differences isolate Planet.
- Every run preserves raw output, argv, hashes, PID/FILETIME, samples and normal
  close/exit evidence. Deadline force, failed parity and missing samples remain
  failed or unavailable evidence. No user process, cleanup or maintenance action
  participates.

The ignored runners are `dist/performance/cleanup-earth/provider-golden-final.py`,
`measure-clean-pairs.py`, and `measure-earth-process-pairs.py`, with the existing
`native-earth-resources.mjs` driver. Each requires `--candidate`,
`--candidate-label`, `--candidate-sha256`, and `--candidate-source-sha256`.
Outputs must use new directories. This preparation ran static syntax inspection
only; it did not execute any measurement, product test, build or app.

## Launch template

Run from the repository root after the final candidate gate and exclusive-slot
handoff. This C3-final example is documentation, not an execution record. If C4
is approved, replace all four frozen identity inputs together before launch;
do not reuse an output directory or relabel earlier results.

```powershell
$measureBase = 'dist/performance/cleanup-earth'
$candidateLabel = 'C3-final'
$candidateDirectory = "$measureBase/$candidateLabel"
$candidateCliSha = '3c44b362b2d0a6b8440a41e66cca36f2613395f695bbeb6b7c68ebd9ecc9a67a'
$candidateNativeSha = '7426037283a60829ee1d124bbd5433c6c0a6005cea307a9df425d1c98cfa85b6'
$candidateSourceSha = '1bce947fad5ef895a211621217b0436d265308d57b59912b1aa6a96701f984b3'
$finalOutput = "$measureBase/final-$candidateLabel-01"
$cliIdentity = @('--candidate', "$candidateDirectory/devsweep.exe", '--candidate-label', $candidateLabel,
    '--candidate-sha256', $candidateCliSha, '--candidate-source-sha256', $candidateSourceSha)

python "$measureBase/provider-golden-final.py" prepare "$finalOutput/providers" @cliIdentity
if ($LASTEXITCODE) { throw 'Provider preparation failed' }
python "$measureBase/provider-golden-final.py" run "$finalOutput/providers" @cliIdentity
if ($LASTEXITCODE) { throw 'Provider parity failed' }

foreach ($referenceLabel in @('B3', 'A')) {
    python "$measureBase/measure-clean-pairs.py" --reference "$measureBase/$referenceLabel/devsweep.exe" `
        --reference-label $referenceLabel @cliIdentity --fixtures "$measureBase/fixtures" `
        --out "$finalOutput/cli-$referenceLabel" --workload many-100 --workload overlap `
        --workload wide --workload deep --workload 'Unicode path 测试'
    if ($LASTEXITCODE) { throw "CLI series failed: $referenceLabel" }
}

python "$measureBase/measure-earth-process-pairs.py" --reference "$measureBase/A/devsweep-desktop.exe" `
    --candidate "$candidateDirectory/devsweep-desktop.exe" --candidate-label $candidateLabel `
    --candidate-sha256 $candidateNativeSha --candidate-source-sha256 $candidateSourceSha `
    --fixtures dist/native-clean-cwd --out "$finalOutput/earth-A" `
    --node 'C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe'
if ($LASTEXITCODE) { throw 'Earth series failed' }
```

The provider closeout additionally verifies the recorded exact CLI identities
and fixed provider executable paths have no survivors before timed work starts.
The empty native cwd is preexisting and contains no selected cleanup roots.
