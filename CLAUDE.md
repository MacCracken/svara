# svara — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** —
> durable rules that change rarely. Volatile state (current version,
> module line counts, port progress, test counts, consumers) lives in
> [`docs/development/state.md`](docs/development/state.md).
> Do not inline state here.

## Project Identity

**svara** — Cyrius port of a Rust project (8785 lines preserved at `rust-old/`).

- **Type**: Port (Rust → Cyrius)
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

## Goal

svara owns **the voice itself** — turning a phoneme and a speaker's
characteristics into audio samples. It is the source-filter synthesizer of the
AGNOS audio stack: glottal excitation, formant filtering, the vocal tract, and
the speech-science layer above them (a 101-phoneme inventory, coarticulation,
prosody, trajectory planning).

What svara deliberately does **not** own: text→phoneme conversion (vansh), audio
I/O and mixing (dhvani), room acoustics (goonj), generic DSP primitives (naad),
and affect modelling (bhava). It takes scalars in and returns samples — no I/O,
no files, no network. Sibling components reach it through the 19 dependency-free
bridge maps in `src/bridge.cyr` rather than by depending on each other. See
[ADR-0006](docs/adr/0006-scope-boundaries.md).

## Current State

> Volatile state lives in [`docs/development/state.md`](docs/development/state.md) —
> port progress, surface parity, in-flight work. Refreshed every release.

This file (`CLAUDE.md`) is durable rules.

## Scaffolding

Project was scaffolded with `cyrius port`. Original Rust at `rust-old/` is the reference oracle — do not modify it; cross-check the port against it.

## Quick Start

```sh
cyrius deps                              # resolve dependencies
cyrius build src/main.cyr build/svara    # compile
cyrius build -D LOGGING src/main.cyr build/svara   # with sakshi tracing (off by default)
cyrius test                              # run tests/*.tcyr
./scripts/check-logging.sh               # M-log gate: builds + passes BOTH ways
```

## Key Principles

- **Cross-check against `rust-old/`** — the port's correctness bar is "matches what Rust did". Diverge only with an ADR.
- **Correctness over cleverness** — if the Cyrius behavior diverges silently from Rust, the bugs win
- Test after every change, not after the feature is "done"
- ONE change at a time — never bundle unrelated changes
- Build with `cyrius build`, not raw `cat file | cc5` — the manifest auto-resolves deps
- Source files only need project includes — stdlib auto-resolves from `cyrius.cyml`
- `var buf[N]` = N **bytes**, not N entries

## Rules (Hard Constraints)

- **Do not commit or push** — the user handles all git operations
- **Never use `gh` CLI** — use `curl` to the GitHub API if needed
- Do not modify `rust-old/` — it's the parity oracle
- Do not skip tests before claiming changes work
- Do not modify `lib/` files (vendored stdlib / dep symlinks)
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth

## Documentation

- [`docs/adr/`](docs/adr/) — Architecture Decision Records (*why X over Y?*)
- [`docs/architecture/`](docs/architecture/) — Non-obvious constraints
- [`docs/guides/`](docs/guides/) — Task-oriented how-tos
- [`docs/examples/`](docs/examples/) — Runnable examples
- [`docs/development/state.md`](docs/development/state.md) — Live state
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — Milestones through v1.0

