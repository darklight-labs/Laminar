# Laminar

Laminar Tracer Bullet (v0.0.1-alpha) is a minimal, end-to-end steel thread for CSV -> parse -> validate -> construct intent -> output. It enforces zatoshi-precision arithmetic and dual-mode output for human operators and automated agents.

Laminar constructs payment intents. It does not hold keys and does not broadcast transactions.

## What This Repo Is
- A deterministic CLI and core library for CSV batch parsing and intent construction
- A tracer bullet that proves zatoshi-only math and dual-mode output

## What This Repo Is Not
- A wallet
- A transaction broadcaster
- A QR/UR generator

## Scope (This Repo)
- CSV ingest and validation
- ZEC decimal parsing into zatoshis (u64, no floats)
- Intent construction as JSON
- Dual-mode CLI output (TTY vs non-interactive)

## Workspace Layout
- `laminar-core`: parsing, validation, shared types
- `laminar-cli`: CLI with human vs agent modes
- `demo/`: sample CSVs and scripts

## Documentation
- [INVARIANTS.md](./INVARIANTS.md): Non-negotiable rules and tracer-bullet subset
- [ARCHITECTURE.md](./ARCHITECTURE.md): Current architecture and data flow
- [CONSTANTS.md](./CONSTANTS.md): Reference constants used in this repo
- [RFC-001.md](./RFC-001.md): Tactical spike scope and current status
- [ROADMAP.md](./ROADMAP.md): Future scope and phased vision
- [THREAT_MODEL.md](./THREAT_MODEL.md): Threats and mitigations for the tracer bullet
- [SECURITY.md](./SECURITY.md): Vulnerability reporting policy
- [CONTRIBUTING.md](./CONTRIBUTING.md): Development workflow and PR checklist
- [demo/README.md](./demo/README.md): Demo assets overview

## Repository Map
- [Cargo.toml](./Cargo.toml): Workspace definition.
- [Cargo.lock](./Cargo.lock): Locked dependency graph for reproducible builds.
- [.gitignore](./.gitignore): Ignores build artifacts and local test outputs.
- [.gitattributes](./.gitattributes): Normalizes text line endings (LF for shell scripts).
- [laminar-core/Cargo.toml](./laminar-core/Cargo.toml): Core crate manifest.
- [laminar-core/src/lib.rs](./laminar-core/src/lib.rs): Core module exports.
- [laminar-core/src/types.rs](./laminar-core/src/types.rs): Shared data types and intent schema.
- [laminar-core/src/output.rs](./laminar-core/src/output.rs): Human/agent output helpers and formatting.
- [laminar-core/src/parser.rs](./laminar-core/src/parser.rs): ZEC decimal parsing to zatoshis.
- [laminar-core/src/validation.rs](./laminar-core/src/validation.rs): Address validation rules.
- [laminar-cli/Cargo.toml](./laminar-cli/Cargo.toml): CLI crate manifest.
- [laminar-cli/src/main.rs](./laminar-cli/src/main.rs): CLI entry point and dual-mode behavior.
- [demo/payroll.csv](./demo/payroll.csv): Valid sample batch.
- [demo/invalid.csv](./demo/invalid.csv): Invalid sample batch for fail-fast validation.
- [demo/run_demo.sh](./demo/run_demo.sh): End-to-end demo script.
- [demo/agent_test.sh](./demo/agent_test.sh): Agent-mode regression checks.

## Dual-Mode CLI
The CLI adapts based on execution context:

| Mode | Trigger | Behavior |
|------|---------|----------|
| Human | Terminal (TTY) | Spinners, tables, colors, confirmations |
| Agent | Piped or `--output json` | Silent, strict JSON, non-interactive |

## Setup (Beginner-Friendly)
These steps assume you have not used GitHub or Rust before. If you are already comfortable with Rust, feel free to jump to the **Build** section.

### 1) Get the code
Pick one of the two options below.

**Option A: GitHub Desktop (no command line)**
1. Install GitHub Desktop from the official GitHub site.
2. Click **File → Clone Repository…**
3. Paste the repository URL and choose a local folder.
4. Click **Clone**.

**Option B: Command line (Git)**
1. Install Git from the official site.
2. Open a terminal and run:
```bash
git clone <REPO_URL>
cd laminar
```

### 2) Install Rust (required)
Laminar is written in Rust. Install the stable toolchain:
- **Windows / macOS / Linux**: Use `rustup` from the official Rust site.
- After install, open a new terminal and confirm:
```bash
rustc --version
cargo --version
```

### 3) Optional: Bash scripts on Windows
The demo scripts in `demo/` are written for bash.
- If you are on Windows, install **Git Bash** or **WSL** to run them.
- If you prefer not to use bash, you can still run everything using the Rust commands in this README.

### 4) Build and test (recommended)
```bash
cargo build --release
cargo test
```

If those commands succeed, your setup is complete.

## Quick Start (Experienced Users)
```bash
git clone <REPO_URL>
cd laminar
rustup default stable
cargo build --release
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force
```

## Build
```bash
cargo build --release
```

## Test
```bash
cargo test
```

## Lint
```bash
cargo clippy -- -D warnings
```

## Run (Human Mode)
Human mode activates when stdout is a TTY.
```bash
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force
```

If you omit `--force`, you will be prompted to confirm before intent construction.

## Run (Agent Mode)
Agent mode activates when stdout is a pipe or when `--output json` is set.

Forced JSON:
```bash
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json --force
```

Auto via pipe:
```bash
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --force | cat
```

Agent-mode confirmation guard (expected error/exit code 2):
```bash
cargo run --release -p laminar-cli -- --input ./demo/payroll.csv --output json
```

## Fail-Fast Validation
Invalid batch should emit JSON error and exit code 1:
```bash
cargo run --release -p laminar-cli -- --input ./demo/invalid.csv --output json --force
```

## CSV Format
Input CSV requires a header row with these columns:
- `address`: recipient address (ASCII only; network-aware prefix validation in this tracer bullet)
- `amount`: decimal ZEC string (up to 8 decimals)
- `memo`: optional memo string

Example:
```csv
address,amount,memo
u1qexample...,10.50,January payroll
```

## Demo Scripts (bash)
```bash
./demo/run_demo.sh
./demo/agent_test.sh
```

## Windows cmd equivalents
If you are using `cmd.exe`, replace `cat` with `more` and use backslashes in paths:
```cmd
cargo run --release -p laminar-cli -- --input .\demo\payroll.csv --force | more
```

## Project Status
This repository is a tracer bullet for the Laminar concept. It focuses on correctness and deterministic output rather than full wallet integration or QR/UR encoding.

## License
Dual-licensed under MIT and Apache 2.0.
