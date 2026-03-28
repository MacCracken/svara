# Dependency Watch

Track upstream dependencies for updates, security advisories, and compatibility.

## Direct Dependencies

| Crate | Pinned | Latest | Notes |
|-------|--------|--------|-------|
| hisab | 1.2 | - | Math (FFT, interpolation, easing). Features: num, calc |
| naad | 1.0 | - | DSP backend (optional). Filters, noise, LFOs |
| libm | 0.2 | - | no_std math fallback |
| serde | 1 | - | Serialization (derive + alloc) |
| thiserror | 2 | - | Error derive (no_std compatible) |
| tracing | 0.1 | - | Structured logging (optional) |

## Dev Dependencies

| Crate | Pinned | Notes |
|-------|--------|-------|
| criterion | 0.5 | Benchmarking with HTML reports |
| serde_json | 1 | Roundtrip test fixtures |

## Security Monitoring

- `cargo audit` runs in CI on every push
- `cargo deny check` validates license compatibility and known advisories
- `codecov.yml` enforces 80% coverage threshold

## Upgrade Policy

- **Patch versions**: Auto-merge if CI passes
- **Minor versions**: Review CHANGELOG, run benchmarks before merging
- **Major versions**: Full review, test migration path, update MSRV if needed
