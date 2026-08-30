# Run2 process exit observation

- Orchestrating execution API result: `exit_code=1`.
- Child helper output recorded in the authentic transcript:
  `HELPER_EXIT_CODE=1`.
- The transcript's `EXIT_CODE=99` is the wrapper's unchanged initialization
  sentinel and is incorrect because the catch block threw before assigning the
  helper failure to `$wrapperExit`.
- The transcript was not edited. This post-run note records the discrepancy; it
  does not replace or repair raw command evidence.
