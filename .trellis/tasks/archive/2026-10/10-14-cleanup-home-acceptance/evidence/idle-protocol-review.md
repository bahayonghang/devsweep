# Final no-debugger idle: protocol review

The declared 548 s run completed on final desktop SHA256
`6904d6bb2d5204f76060226257957e7aa5ab9933c37bf5528efb5b7cd3e88feb`.
PID 79764 / creation FILETIME 134364730814324698 stayed the host identity.
The launch and summary both have no debug port. Raw evidence is under
`dist/cleanup-home-final/native-idle-final-01/`.

## Protocol coverage

The parent inspected all five windows, independently of the analyzer's numeric
PASS. Each nominal 60 s retained interval has 292–293 samples. Its first sample
arrives within 154 ms of the start; its last sample is within 76 ms of the end.
Observed spans are 59.772–59.913 s and maximum intersample gaps are
210.182–210.670 ms, consistent with the nominal 200 ms sampler plus its read
overhead. No window is reduced to a handful of samples or missing counters.
This matches the frozen five-window protocol's actual sampling coverage.

Normal WM_CLOSE was requested at 548,007.149 ms. Exit code is 0; helpers settled
in 241.541 ms, with no forced teardown or surviving owned processes. The fixture
sentinel hashes are unchanged and no cleanup was executed.

## Numeric host gates and separate tree concern

The inherited **host-PID** limits pass: median of five CPU medians 0.0% at the
counter resolution (limit 1%), maximum private bytes 5,234,688 (4.99 MiB; limit
192 MiB), maximum 14 threads (limit 40). This is not a zero-physical-work claim.

**Do not describe the entire application as low idle load.** The complete owned
tree shows 70.08–82.49% integrated CPU per retained window, where 100% denotes
one logical core, not the whole 24-logical-processor machine. Tree peak private
memory is 264,953,856 bytes, peak thread count 253, and peak process count 8.
Tree identities remain stable inside each retained window, so these are actual
available deltas rather than missing-counter zeros.

`per-process-diagnostic.json` attributes almost all of that CPU to child
73556 / creation FILETIME 134364732016977045, appearing about 120.27 s after the
host starts. Its role is not inferred from the generic msedgewebview2 name.
Owned command-line capture and matched 180 s diagnostics are complete; see
`../matched-native-diagnostics.md`. Neither the current nor earlier candidate
run reproduced the late child. Both contain the new Clean UI, so this does not
isolate the redesign against the pre-edit baseline. The hot child's role and
the cause remain unknown. All diagnostic processes closed normally with
unchanged sentinels. The formal raw run remains unchanged. This concern does not retroactively
invent or waive a tree budget; it does prevent an unsupported whole-app idle
success claim and keeps runtime investigation open.
