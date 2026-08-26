# Security Policy

## Reporting a Vulnerability

1. **Do not** open a public issue.
2. Email security@agnos.org with a description and reproduction steps.
3. Please allow 90 days for a fix before public disclosure.

svara is a **library** with no I/O, network or filesystem access. It turns
untrusted numeric parameters into audio buffers, and it is compiled *into* the
consumer's binary as a source bundle — so there is no load-time trust boundary.
The attack surface, and what is and is not mitigated, is
[`docs/development/threat-model.md`](docs/development/threat-model.md).

## Supported versions

| Version | Supported | Notes |
|---|---|---|
| **3.5.x** | ✅ Yes | current |
| 3.2 – 3.4 | ⚠ Upgrade | no known reachable defects, but missing later hardening |
| 3.0 – 3.1.2 | ❌ **No** | five defects reachable through the public API — see below |
| 1.x, 2.x | ❌ No | the Rust line, superseded by the Cyrius port at 3.0.0 |
| 0.1.x | ❌ No | port scaffold |

> The previous version of this file said "1.x: Yes" and listed nothing else. That
> was inherited from the Rust crate and was materially misleading by the time
> 3.1.3 shipped.

## Known defects in 3.0.0 – 3.1.2 — upgrade if you are on these

An adversarial-input sweep in **3.1.3** found five defects reachable through the
public API with **no unsafe usage by the caller**. They are recorded in
[ADR-0001](docs/adr/0001-signed-index-and-float-conversion-hazards.md) and pinned
by `tests/hardening.tcyr`.

| Defect | Effect | Reproducer |
|---|---|---|
| Unchecked `alloc` failure | **SIGSEGV** — `alloc` returns 0, the next `store64` writes to address 0 | `svara_phoneme_synthesize(VowelA, voice, 44100.0, 6100.0)` |
| Negative spectrum bin | **Process abort** (`vec: index < 0`) ×3 | `svara_spectrum_magnitude_at` / `_band_energy` / `_peak_in_range` with a negative or NaN frequency |
| Inverted NaN range test | **Silent NaN output** — a NaN `f0` was accepted and reported success | `svara_glottal_set_f0(g, NaN)` |
| Error code stored as a pointer | Dereference of a small negative address | `svara_tract_new` with a non-positive or non-finite sample rate |
| Unbounded doubling loop | **Hang** | `svara_next_pow2` past 2³² |

Their root cause is one class: Rust encodes guarantees in its type system that
Cyrius has no equivalent for — saturating `as` casts, unsigned indices, an
aborting allocator, and `panic!` — and the port dropped them silently because the
ported line *looks* correct.

**Measured, not asserted:** 9 of `tests/hardening.tcyr`'s 10 public-API probes
fail against a 3.1.2 checkout (four wrong value, three process abort, one
SIGSEGV, one hang), while the control passes on both sides.

## Later hardening

- **3.2.0** — `#derive(Serialize)` has no skip attribute, so a serialized struct
  may not hold a scratch pointer; one would leak a heap address (an ASLR
  disclosure) into JSON that may cross a trust boundary, and return dangling.
  Enforced by construction, see
  [ADR-0002](docs/adr/0002-serialization-boundary.md).
- **3.3.0** — every state companion's `restore` **validates** its input rather
  than trusting it; a state that has been through JSON is externally-authored
  data.
- **3.5.2** — NaN formant parameters are rejected
  ([ADR-0007](docs/adr/0007-reject-nan-formant-parameters.md)). Previously they
  were accepted for parity with the Rust oracle, and the filter then emitted an
  entire buffer of NaN **having returned success** — a plausible-looking `Ok`,
  which is the hardest failure shape for a caller to notice.

## What svara does not defend against

- **Unbounded `duration` from untrusted input.** svara refuses the *impossible*
  (past `SV_MAX_SAMPLES`, ≈101 minutes at 44.1 kHz) but not the merely enormous:
  a 90-minute render is accepted and allocates ~1.9 GB. **Bound it yourself.**
  This matters more in Cyrius than it would elsewhere — the bump allocator never
  frees, so memory returns to the OS only at process exit.
- **A malicious consumer.** svara is a library in the consumer's address space,
  not a sandbox.
- **Cryptographic use of `SvRng`.** It is PCG32, chosen for reproducibility, and
  drives jitter/shimmer/aspiration noise. Never use it as a source of secrets.

## Supply chain

Dependencies are **entirely git-based** — there is no registry. `cyrius.lock`
pins every dependency to a resolved commit SHA and records a SHA-256 for every
vendored file in `lib/`. The toolchain installs through the upstream
`scripts/install.sh`, which verifies a published SHA-256 (CVE-21) and, where
present, an Ed25519 signature over `SHA256SUMS` (CVE-13). CI runs `cyrius deny`
and asserts the installed toolchain matches the manifest pin.

⚠ **Vendored bundles are compiled as svara's own source**, not linked. A
compromised `lib/*.cyr` is arbitrary code execution in the consumer's process;
the lockfile hashes are what detect tampering.
