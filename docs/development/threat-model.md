# Threat Model

## Scope

svara is a library crate that processes untrusted input parameters and produces
audio sample buffers. It does not perform I/O, network access, or file operations.

## Attack Surface

### 1. Parameter Injection

**Threat**: Malicious or malformed parameters (NaN, Inf, extreme values) passed
to public API functions could cause panics, infinite loops, or memory corruption.

**Mitigations**:
- All constructors validate sample_rate (positive, finite)
- All synthesis functions validate duration (positive, finite)
- GlottalSource validates f0 range [20, 2000] Hz
- FormantFilter validates frequency < Nyquist, bandwidth > 0
- Clamp-based setters prevent out-of-range values
- Zero unwrap/panic in library code (enforced by clippy + review)

### 2. Denial of Service

**Threat**: Extremely large duration or sample count causing excessive memory
allocation or CPU consumption.

**Mitigations**:
- `synthesize_into()` takes caller-owned buffers (caller controls size)
- `synthesize()` allocates based on `duration * sample_rate` — callers should
  bound duration input before calling
- No recursive algorithms that could stack overflow

### 3. Information Disclosure

**Threat**: Serialized state (via serde) could leak internal PRNG state,
enabling prediction of future random values.

**Assessment**: Low risk. PRNG state is not cryptographic — it's used for
jitter/shimmer/noise which are audible by design. No secrets in the state.

### 4. Supply Chain

**Threat**: Compromised dependencies introducing malicious code.

**Mitigations**:
- `cargo deny check` in CI validates licenses and advisories
- `cargo audit` checks for known CVEs
- Minimal dependency tree (6 direct deps)
- `deny.toml` restricts to crates.io only (no git dependencies)

## Trust Boundaries

- **Untrusted**: All public API parameters (f0, duration, sample_rate, formants)
- **Trusted**: Internal module boundaries, private functions
- **External**: hisab and naad crate outputs (validated at integration points)
