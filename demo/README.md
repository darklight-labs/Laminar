# Demo Assets

This folder contains sample CSVs and bash scripts used to validate the tracer-bullet flow.

## Files
- `payroll.csv`: valid sample batch with various decimal formats.
- `invalid.csv`: invalid rows to exercise fail-fast validation and error aggregation.
- `run_demo.sh`: runs human and agent modes in sequence.
- `agent_test.sh`: agent-mode checks for exit code handling, JSON fields, and determinism.

## Notes
- Scripts require bash (Git Bash or WSL on Windows).
- If you are using Windows `cmd.exe`, see the root `README.md` for equivalent commands.
