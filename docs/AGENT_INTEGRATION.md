# Laminar Agent Integration Guide

## Overview
Laminar CLI supports two modes in one binary:

- `operator` mode: human-friendly terminal UX.
- `agent` mode: deterministic JSON for automation.

For integrations, always use agent mode.

## Mode Detection
Mode is selected deterministically:

1. `--output json` => `agent`
2. `--interactive` => `operator`
3. otherwise:
- TTY stdout => `operator`
- non-TTY stdout (pipe/redirect) => `agent`

Agent mode invariant:

- it never blocks on stdin
- confirmation is implicitly approved

## CLI Commands
## Validate
```bash
laminar-cli --output json validate ./batch.csv --network mainnet
```

## Construct
```bash
laminar-cli --output json construct ./batch.csv --network mainnet --output-dir ./out
```

## Generate
```bash
laminar-cli --output json generate ./batch.csv --network mainnet --output-dir ./out
```

## Response Schema
All agent responses are JSON objects written to stdout.

```json
{
  "error": null,
  "laminar_version": "0.1.0",
  "mode": "agent",
  "operation": "generate",
  "result": {
    "batch_id": "uuid-or-null",
    "network": "mainnet",
    "qr_files": ["./out/qr-static.png"],
    "receipt_file": "./out/laminar-receipt-2026-02-11-12345678.json",
    "recipient_count": 10,
    "segments": 1,
    "total_zatoshis": 123456789,
    "total_zec": "1.23456789",
    "ur_encoded": null,
    "zip321_uri": "zcash:..."
  },
  "success": true,
  "timestamp": "2026-02-11T08:00:00Z",
  "warnings": []
}
```

Behavior details:

- keys are sorted alphabetically (recursive)
- `result` is `null` on hard failures
- `error` is `null` on success
- `warnings` is `null` or array depending on command/output path

## Error Handling
On failure:

- process exits non-zero
- JSON includes:
  - `success: false`
  - `error.code`
  - `error.name`
  - `error.message`
  - `error.details[]`

Recommended handling:

1. trust process exit code for control flow
2. parse `error.code` for machine decisions
3. surface `error.message` + `error.details` for observability

## Exit Codes
- `0`: success
- `1`: validation error
- `2`: config error
- `3`: I/O error
- `4`: internal error
- `10`: confirmation required
- `11`: stdin blocked

In proper agent mode (`--output json`), `10` and `11` should not occur.

## Python Integration Example
```python
import json
import subprocess
from pathlib import Path

cmd = [
    "laminar-cli",
    "--output", "json",
    "generate",
    "test-vectors/valid-simple.csv",
    "--network", "mainnet",
    "--output-dir", "out",
]

proc = subprocess.run(
    cmd,
    text=True,
    capture_output=True,
    check=False,
    timeout=15,
)

if not proc.stdout.strip():
    raise RuntimeError(f"No JSON on stdout. stderr={proc.stderr}")

response = json.loads(proc.stdout)

if proc.returncode != 0 or not response.get("success", False):
    err = response.get("error") or {}
    raise RuntimeError(
        f"Laminar failed rc={proc.returncode} "
        f"code={err.get('code')} name={err.get('name')} msg={err.get('message')}"
    )

print("ZIP321:", response["result"]["zip321_uri"])
print("Receipt:", response["result"]["receipt_file"])
```

## Security Considerations
- Treat input files as sensitive operational data.
- Validate network (`mainnet` vs `testnet`) before execution.
- Store receipts and generated artifacts in controlled paths.
- Prefer offline/isolated execution for high-value batches.
- Pin Laminar version in automation and monitor release notes.

## Integration Checklist
1. Always pass `--output json`.
2. Capture stdout and parse JSON.
3. Check process exit code and `success`.
4. Handle retry/abort policies based on `error.code`.
5. Persist `result.receipt_file` and `result.zip321_uri` for traceability.
