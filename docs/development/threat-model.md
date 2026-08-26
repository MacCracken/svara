# Threat Model

> Rewritten 2026-08-26 for the Cyrius port. The pre-port version described a Rust
> crate — `cargo deny`, `cargo audit`, a `deny.toml` restricting to crates.io —
> and asserted parameter-validation mitigations that the 3.1.3 audit disproved.
> Both are corrected below. Prerequisite for the v1.0 security audit
> (`docs/audit/`).

## Scope

svara is a **library** that turns untrusted numeric parameters into audio sample
buffers. It performs no I/O, no network access and no filesystem access; the only
resource it consumes is memory, from the process-wide bump allocator.

It is compiled and distributed as `dist/svara.cyr`, a source bundle the consumer
compiles into their own binary. There is no separate trust boundary at load time
— svara's code becomes the consumer's code.

## What changed the model: Cyrius has no type-system safety net

The Rust original leaned on guarantees Cyrius does not have, and the port dropped
them silently because the ported line looks correct. This is documented in full in
[ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md); the
security-relevant summary:

- **No bounds checking on raw memory.** `load64` / `store64` take an address.
  `vec_get` / `vec_set` *do* check and abort, so which primitive an index reaches
  determines whether a bad index is a clean crash or silent corruption.
- **`alloc` returns 0 on failure**, it does not abort. An unchecked allocation
  followed by a write is a write to address 0 and upward.
- **`f64_to` does not saturate.** NaN, ±inf and out-of-range all yield
  `INT64_MIN`, where Rust's `as` casts give 0 or `MAX`.
- **No unsigned types.** A Rust bounds check written as one `<` test on a `usize`
  is only half a bound in signed `i64`.
- **No panic.** A Rust `.expect()` has no equivalent unless one is written.

## Attack surface

### 1. Parameter injection — *previously overstated*

**Threat**: malformed parameters (NaN, ±inf, extreme or negative values) passed
to the public API cause memory corruption, a process abort, or silently wrong
output.

⚠ **The pre-port version of this document claimed these were mitigated. In 3.1.3
an adversarial sweep found five defects reachable through the public API with no
unsafe usage by the caller** — one SIGSEGV, three process aborts and one silent
NaN. The claim was written for the Rust and inherited without being re-tested.

**Mitigations, as they now actually stand:**

- Every f64 → count/index cast goes through `svara_count_from_f64`, which
  reproduces Rust's saturating `as usize` (NaN/negative → 0; over-range → past
  the cap, which every downstream test rejects).
- Every size-driven allocation goes through `svara_alloc_samples`, which bounds
  the count *before* multiplying, and **every caller checks the 0 return**.
- Index producers saturate at the source — `svara_spectrum_frequency_bin` returns
  a non-negative bin, exactly as Rust's `-> usize` did, so its callers' single
  upper-bound tests are complete bounds again.
- Range rejections are written **positively** (`if (in_range(x) == 0) reject`) so
  NaN falls through to the reject, matching Rust's `!(a..=b).contains(&x)`.
- Constructors that cannot build their object return a negative `SVARA_ERR_*`
  rather than storing an error code where a pointer belongs.
- `restore` on every state companion validates its input: a state that has been
  through JSON is externally-authored data.

**Verification, not assertion**: `tests/hardening.tcyr` pins every instance.
Its discrimination is measured — 9 of 10 public-API probes fail against a 3.1.2
checkout (4 wrong value, 3 process abort, 1 SIGSEGV, 1 hang) while the control
passes on both sides. Re-measure when the group is extended.

**Known residual, carried deliberately**: `svara_formant_validate` accepts a NaN
formant frequency, producing NaN coefficients and NaN output. Rust's
`f.frequency <= 0.0 || f.frequency >= nyquist` accepts it too, so this is parity,
annotated at `src/formant.cyr`. It is silent-wrong-output, not memory-unsafe.
Tightening it is a deliberate divergence and needs its own ADR.

### 2. Resource exhaustion

**Threat**: a large duration or sample count consumes excessive memory or CPU.

**This is the sharpest remaining risk, because Cyrius's bump allocator never
frees.** Memory is released only at process exit. A leak in a loop is unbounded
growth, not a slow drift.

**Mitigations:**

- `SV_MAX_SAMPLES` (`ALLOC_MAX / 8` = 268,435,456 f64 ≈ 101 minutes at 44.1 kHz)
  caps any single sample buffer. Past it the call returns
  `SVARA_ERR_COMPUTATION` rather than dereferencing a null.
- `svara_tract_synthesize_into` and the `_into` family take caller-owned buffers,
  so the caller controls the size.
- The per-sample synthesis path allocates **nothing**. 3.1.3 removed 1,121 bytes
  per output sample from `svara_sequence_render_planned` (a minute of speech
  needed ~3 GB it could never release); `tests/allocbudget.tcyr` pins the budget
  as marginal arena bytes per sample, with controls proving the measurement is
  real.
- No recursion, so no stack-overflow path.

⚠ **A caller must still bound `duration` from untrusted input.** svara refuses
the impossible, not the merely enormous: a 90-minute render is accepted and will
allocate ~1.9 GB.

### 3. Information disclosure

**Threat A — PRNG state via serialization.** `SvRng` and the state companions
carry PCG32 state. **Assessment: low.** The PRNG is not cryptographic and drives
jitter/shimmer/aspiration noise, all audible by design. There are no secrets in
it. Restoring a saved state deliberately resumes the same draw sequence — that is
the feature.

**Threat B — heap addresses via serialization.** `#derive(Serialize)` has **no
skip attribute**, so every field of a serialized struct is emitted. A raw pointer
in such a struct would leak a heap address (an ASLR disclosure) into JSON that
may cross a trust boundary, and would return dangling.

**Mitigation**: a serialized struct may not hold a scratch buffer — the rule is
stated in [ADR-0002](../adr/0002-serialization-boundary.md) and enforced by
construction: `SvProsodyContour`'s interpolation scratch moved to module level in
3.2.0 for exactly this reason, and the engine types reach JSON only through flat
state companions that contain no pointers.

**Threat C — memory reuse.** `alloc_reset` scrubs the reused span of the first
chunk before rewinding, so a later allocation cannot read the prior occupant's
bytes. svara never calls it; a consumer that does inherits that behaviour from
the stdlib.

### 4. Supply chain

**Threat**: a compromised dependency introduces malicious code. svara's
dependencies are **entirely git-based** — there is no registry — so the integrity
story rests on the toolchain installer and the lockfile.

**Mitigations:**

- `cyrius deny src/main.cyr` validates the dependency set in CI (currently 21
  deps, 0 violations).
- `cyrius.lock` pins every dependency to a **resolved commit SHA**, not just a
  tag, and records a SHA-256 for every vendored file in `lib/`.
- The toolchain is installed through the upstream `scripts/install.sh`, which
  verifies a published SHA-256 (**CVE-21**) and, where present, an Ed25519
  signature over `SHA256SUMS` (**CVE-13**). Both CI workflows use it; neither
  hand-rolls `curl` + `tar`.
- CI asserts the installed toolchain equals the manifest pin before resolving
  deps, so a substituted toolchain fails loudly rather than surfacing as an
  unrelated error downstream.
- 4 direct/transitive git dependencies (hisab, naad, goonj, sakshi), all
  first-party, all pinned to released tags that match what those bundles
  themselves pin.

⚠ **Vendored dependency bundles are compiled as svara's own source.** `lib/*.cyr`
is not a linked artifact — it is concatenated into the build. A compromised
bundle is arbitrary code execution in the consumer's process. The lockfile
hashes are what detect tampering after resolution; `cyrius deps` re-verifies.

### 5. Foreign handles

svara consumes naad's `BiquadFilter`, `NoiseGenerator` and `Lfo` as opaque
handles. Their internal state is not readable, which is why the glottal state
companion cannot reproduce the aspiration stream across a restore
(ADR-0002). Security-wise this is a *containment* property: svara neither
inspects nor serializes them, so no naad internal state reaches svara's output
surface.

## Trust boundaries

| | Trust | Note |
|---|---|---|
| Public API parameters (`f0`, `duration`, `sample_rate`, formants, nasalization) | **Untrusted** | validated per §1 |
| Decoded state (`*_from_json_str`, any `*State`) | **Untrusted** | `restore` validates rather than trusts |
| Caller-supplied buffers (`*_into`) | **Untrusted size** | the caller owns and bounds them |
| Internal module boundaries, private helpers | Trusted | single-namespace library, no isolation available |
| hisab / naad / goonj / sakshi bundles | **Trusted by construction** | compiled as svara's own source; see §4 |

## Non-goals

- **Cryptographic randomness.** `SvRng` is PCG32, chosen for reproducibility.
  Never use it as a source of secrets.
- **Constant-time execution.** Synthesis timing depends on parameters. svara
  processes no secrets, so there is nothing to leak by timing.
- **Sandboxing a malicious consumer.** svara is a library in the consumer's
  address space, not a boundary.
