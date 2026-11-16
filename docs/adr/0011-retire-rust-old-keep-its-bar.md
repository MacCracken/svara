# 0011 — Retire `rust-old/`, and keep its bar

**Status**: Accepted
**Date**: 2026-11-16

## Context

`rust-old/` held the original Rust library as the port's parity oracle: 36 tracked
files, 8,785 lines of source, plus about 445 MB of ignored build output. The 3.6.3
P(-1) sweep was its last planned use. Six cluster audits read every module against
it, and [ADR-0009](0009-what-the-3-6-3-audit-leaves-open.md) records every divergence
they found. Nothing in the build, the suites, the benches, the bundle, CI or
`scripts/` reads it.

"Nothing reads it" was also true of the ten integration tests 3.6.3 had to port, and
of the 25 test gaps 3.6.5 found. So the deletion was checked rather than assumed, on
the model of goonj's ADR-0006:

| Check | Result |
|---|---|
| The gate: a consumer moves its own pins | Five consumers moved to svara 3.6.3 on cyrius 6.7.18 in their own manifests by 2026-11-11: dhvani 2.3.1, prani 2.1.1, hoosh 2.12.3, shabda 3.1.1 and shabdakosh 3.1.1. That is the stricter of the two readings 3.6.2 left open, so the choice between them no longer matters. |
| Every gate without it | `cyrius audit` (25 suites and the benches), `scripts/check-logging.sh`, `cyrius fuzz`, `cyrius deny`, the five examples and `cyrius deps --locked` pass on the release tree with the directory gone, and `cyrius distlib` writes the same bundle. |
| The oracle rebuilds from the tag | `git archive 3.6.5 rust-old`, then `cargo test --all-features --offline --locked`: 165 unit, 45 integration and 3 doc tests pass. |
| Rust tests never ported | All 210 `#[test]`s, plus the 3 doctests, were mapped onto the suites by what each asserts. 3.6.5 ported the 4 with no Cyrius assertion and the 21 with a weaker one, from the Rust. |
| Rust benches with no Cyrius counterpart | 4 of 15: `vocal_tract_reduced_1024`, `vocal_tract_minimal_1024`, `phoneme_render_female_vowel_a` and `sequence_render_10_phonemes`. Their Rust medians are archived in [`benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md). |
| Files found only there | `benches/benchmarks.rs`, `Cargo.toml`, `Cargo.lock`, and the Rust-only tool configs: `deny.toml`, `codecov.yml`, `Makefile`, `rust-toolchain.toml`, `supply-chain/` and `LINES_OF_RUST.txt`. The tag keeps all of them. |
| Citations | All 74 `rust-old/...` path citations in `src/`, `tests/`, `examples/` and `docs/` resolve at the tag. |

## Decision

**Delete the tree from `main`. Keep its bar.** The port's correctness bar is still
"matches what Rust did, and diverge only with an ADR". What changes is how to check it.

- **The Rust stays readable at tag `3.6.5`**, the last tag that has it:
  `git show 3.6.5:rust-old/src/<module>.rs`. Every tag from `3.0.0` to `3.6.5`
  carries `rust-old/`. The `2.0.0` tag is the Rust release itself, with the source
  at the root, and `2.0.0:src/` is byte-identical to `3.6.5:rust-old/src/`.
- **The `rust-old/src/x.rs:NN` citations in `src/`, `tests/` and the ADRs stay.** They
  are the port's audit trail. Since 3.5.2 new citations name a tag. The older bare
  ones resolve at `3.6.5`, which is where to read them.
- **Day to day, the bar is what the oracle left behind:**
  - the parity assertions in `tests/*.tcyr`;
  - `tests/oracle.tcyr`, whose expectations were re-derived from the Rust;
  - the hostile-input groups in `tests/hardening.tcyr`;
  - the divergences recorded in ADR-0001, ADR-0007, ADR-0008, ADR-0009 and ADR-0010.
- **A question only the oracle can answer still gets asked of the oracle.** The 3.9.x
  roadmap items, the preserved Rust quirks, are such questions. To run it again:

  ```sh
  mkdir /tmp/oracle
  git archive 3.6.5 rust-old | tar -x -C /tmp/oracle
  cd /tmp/oracle/rust-old && cargo test --all-features --locked
  ```

  To check a value the Rust tests do not assert, add a throwaway test file to that
  copy and print it, as 3.6.5 did for its controls. Never take the expectation from
  svara's own output.

## Consequences

- **Positive** — The tree is one language. A clone no longer carries Rust that
  nothing builds, and a reader is not invited to edit a frozen oracle.
- **Negative** — A parity question now costs a tag extraction, so it will get asked
  less. New behaviour past the Rust surface never had an oracle, and still does not.
- **Negative** — The rebuild resolves from crates.io. `Cargo.lock`, kept by the tag,
  pins hisab 1.4.0, naad 1.2.5, serde 1.0.228 and serde_json 1.0.150, so a rebuild on
  a machine without a cargo cache depends on those versions staying available.
- **Neutral** — The Rust benchmark figures stay a frozen record in
  `benchmarks-rust-v-cyrius.md`, re-runnable only from the tag.

## Alternatives considered

- **Keep it in the tree** — rejected. It was only ever scheduled to stay until the
  sweeps finished, and a frozen second language invites edits and stale reading.
- **Move it to a branch or a separate repo** — rejected. The tag already does that,
  with nothing to maintain.
- **Commit golden outputs** — rejected. The suites already pin what matters, and f32
  Rust outputs would be re-litigated against every f64 rounding
  ([ADR-0008](0008-the-synthesis-core-stays-f64.md)).
