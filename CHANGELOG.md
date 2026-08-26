# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet.

## [3.5.0] - 2026-08-26 — the redundant delay line, and a roadmap premise that was wrong

**M-perf, the bit-identical half.** The formant bank kept its input delay in
eight per-slot buffers when one scalar pair would do. Removing the redundancy is
worth **−6.0%** on the per-sample formant path and **−4.2%** on the block path,
with **bit-identical output** and no new dependency.

For scale: that is about what the AVX2 prototype bought before it was reverted in
3.1.0 — from the lever the roadmap predicted would work, because it attacks
memory traffic rather than arithmetic.

### Changed — the input delay line is two scalars, not sixteen slots

All eight biquads in the bank are driven by the **same** input sample, so
`x1[i] = input` and `x2[i] = previous input` held identical values in every slot.
Sixteen bytes of state doing the work of two, and 32 memory operations per sample
doing the work of three.

⭐ **Measured before changing anything**, not inferred from reading: over 4,096
samples of real signal, **28,672 cross-slot comparisons found zero divergence**
in `x1` or `x2` — while `y1`, which is genuinely per-slot, differed as expected.
That control is what makes the zero meaningful rather than a check on a buffer of
zeros.

`svara_formant_bank_process` now reads the shared delay once before the loop and
advances it once after. Same arithmetic, same order — the ten render-path digests
are still identical to 3.1.2, and all 840 assertions pass unchanged.

| benchmark | before | after | |
|---|---|---|---|
| formant filter `process_sample` | 168 ns | **158 ns** | **−6.0%** |
| formant filter `process_block` 1024 | 180.4 µs | **172.7 µs** | **−4.2%** |
| tract `process_sample` (Full) | 286 ns | **279 ns** | **−2.4%** |

Minimum of three interleaved runs of each binary, so machine drift hits both
sides equally. A single non-interleaved run had `glottal next_sample` moving
79 → 121 ns on code this release does not touch, which is why the numbers above
are not from one.

Also drops **128 bytes of state per bank** (two 8-slot f64 buffers).

⚠ `SvFormantBank_x1` / `_x2` are gone, replaced by `_xin1` / `_xin2` scalars.
They were derived accessors on an internal delay line; nothing outside
`formant.cyr` read them.

### Fixed — the roadmap asked for a hoist that had nothing to hoist

M-perf listed *"tract per-sample chain — hoist coefficient recompute out of the
per-sample loop"*. There is no coefficient recompute in
`svara_tract_process_sample`: the naad biquads are configured when formants are
set, and the only per-sample state work is the two `SmoothedParam` advances,
which must happen every sample by definition. The item is corrected in the
roadmap rather than left to send someone looking.

### Notes — one change measured, rejected, and recorded

The four quality predicates the tract calls once per sample (`use_interaction`,
`use_nasal_coupling`, `use_subglottal`, `use_lip_radiation`) are one or two
integer comparisons each, so marking them `#inline` looked free. It moved
`tract process_sample` by **+0.7%** — noise, in the wrong direction — over the
same interleaved three-run protocol.

Reverted. The source carries no pragma that does nothing, and `src/lod.cyr` now
records the negative result so nobody re-tries it without measuring. The call
overhead is not what that loop spends its time on.

### Still open in M-perf

Unchanged and now the whole of the remaining lane: pooling the per-note render
buffers (unblocked), and three items blocked outside this repo — a stdlib
`vec_with_capacity`, a non-allocating hisab `calc_monotone_cubic`, and the SIMD
work waiting on Cyrius codegen that does not round-trip every `f64v4` op through
memory.

- `dist/svara.cyr` regenerated at v3.5.0.

## [3.4.0] - 2026-08-26 — structured logging through sakshi

**M-log.** svara emitted no diagnostics at all — errors were integer codes and
nothing else. It now routes tracing through **sakshi**, the AGNOS structured
logging substrate, so its spans nest inside a consumer's and correlate across
the stack. This also completes the sakshi routing varna's own `logging.cyr`
explicitly deferred, approximating it with a level-gated stderr writer.

**Off by default, and compiled out entirely when off** — no log call, no runtime
level check, no call frame. Suite **828 → 840 assertions** (21 suites), plus
**58** more that only exist in the logging build.

```sh
cyrius build -D LOGGING src/main.cyr build/svara
SVARA_LOG=debug ./build/svara
```

### Added — `src/logging.cyr`

`svara_log_init()` reads `SVARA_LOG` (`off`/`fatal`/`error`/`warn`/`info`/
`debug`/`trace`, default `info`); an unrecognised value falls back to `info`
rather than silencing a program that asked for logs. Level-gated
`svara_log_error`/`_warn`/`_info`/`_debug`/`_trace`, structured `svara_log_kv`,
and `svara_log_err`, which binds a `SVARA_ERR_*` to the active span via
`sakshi_err_at_span` — the span depth becomes the error's context, so a consumer
can tell *which nesting level* failed.

Instrumented: **four coarse entry points** — `svara_sequence_render`,
`svara_batch_render_all`, `svara_synthctx_synthesize`, `svara_pool_render`.
**Never per-sample**: a span costs two `clock_gettime` calls, which at 44.1 kHz
would dominate the ~0.56 µs it takes to synthesize a sample.

### Added — token-based spans, because every entry point has early returns

⭐ **A plain `span_enter` / `span_exit` pair would leak a span on every error
return**, and sakshi's stack is 16 deep — so a leak per failed render would
slowly poison the log rather than fail loudly. `svara_synthctx_synthesize` alone
has seven returns.

`svara_span_enter` returns the depth to unwind *to*, and `svara_span_leave(token)`
unwinds to it — correct even if an inner span leaked, and harmless called twice.

Asserted, not assumed: **40 consecutive failing renders leave the span stack at
depth 0**, which is well past the 16 that would overflow it. Each error exit is
covered separately, including the pre-span argument rejections that correctly
open no span at all.

### Fixed — `SVARA_LOG=off` now means off, spans included

⚠ **sakshi does not level-gate span events.** `_sk_emit_span` writes regardless
of `sakshi_set_level`, because a span is structural rather than a log line. That
is defensible in sakshi and wrong here: setting `off` still streamed every
ENTER/EXIT pair. svara gates spans itself.

Asserted with the ring buffer: a whole `sequence_render` at level `off` emits
**zero** events, and the *same* render at `debug` emits some — so the silence is
the level, not a broken sink.

### Added — `scripts/check-logging.sh`, and CI runs it

⚠ **`cyrius test` does not forward `-D`.** So `cyrius audit` only ever compiles
the logging-OFF half, and without this script the entire `#ifdef LOGGING` half of
the codebase — the module and the guarded blocks through four entry points —
**would be built by nobody**.

The script builds and runs both ways, requires both builds to emit no `warning:`
at all, and asserts structurally that **no log or span call appears on the
per-sample path** (`svara_glottal_next_sample`, `svara_tract_process_sample`,
`svara_formant_bank_process`, …). That last check was verified to discriminate by
injecting a `svara_log_trace` into `svara_formant_bank_process` and confirming it
fails.

### Notes

- **`tests/logging.tcyr` is useful in both modes.** With logging off it asserts
  the four instrumented entry points still behave exactly as before — which is
  what proves the `#ifdef` blocks threaded through their return paths changed
  nothing. Benchmarks agree: every row is within noise of 3.3.2, and the ten
  render-path digests are still identical to 3.1.2.
- ⚠ **`dist/svara.deps` now names sakshi**, because the bundle carries
  `src/logging.cyr`. This is **not a new dependency** — sakshi already arrived
  transitively through hisab, which svara's bundle requires — but it is now
  explicit. `docs/development/integration-guide.md` shows the stanza.
- **The lint rule I documented in 3.3.1 caught this release's own module**: the
  header used the word "deferred" without a tracking pointer on the same line,
  and `cyrlint` flagged it. Fixed by citing the roadmap where the word appears.
- `dist/svara.cyr` regenerated at v3.4.0 (5,728 lines).

## [3.3.2] - 2026-08-26 — glottal restore is exact; a 3.3.0 limitation was self-inflicted

⭐ **3.3.0 shipped a documented limitation that was not real.** `SvGlottalState`
said naad's `NoiseGenerator` cell and `Lfo` phase "are internal to naad and
cannot be read out", so a restored source restarted its aspiration and vibrato
streams — and `tests/serde.tcyr` *asserted that divergence*, which made a wrong
belief look like a measured fact.

naad puts `#derive(accessors)` on both structs, and svara links against naad's
bundle in **one flat namespace**. The state was reachable the whole time:
`NoiseGenerator_rng` and `Lfo_rng` return pointers to 8-byte PRNG cells, and
`Lfo_phase` / `Lfo_sh_value` are ordinary fields. Verified by probe before
changing anything — two generators given the same cell produce 64 identical
samples.

**What a foreign type lacks is a codec, not accessors.** That is the
generalisable lesson, and it is now recorded in
[ADR-0002](docs/adr/0002-serialization-boundary.md).

### Fixed — `svara_glottal_restore` is now exact in every configuration svara builds

`SvGlottalState` gains four fields — `noise_cell`, `lfo_phase`, `lfo_sh_value`,
`lfo_rng_cell` — captured in `save` and written back in `restore` *after*
`svara_glottal_set_vibrato`, which would otherwise leave the LFO at its
construction phase.

The test group flipped from asserting divergence to asserting identity: a
restored source now matches **exactly over 512 samples with breathiness at 0.5
and vibrato at 5 Hz**, where 3.3.0 asserted it must differ. A control confirms a
*fresh* source does not match, so the capture is doing the work rather than the
comparison being vacuous.

⚠ **Complete for the generators svara constructs** — a WHITE `NoiseGenerator`,
whose entire state is that one cell, and a SINE `Lfo`. Pink noise keeps an octave
**buffer** (`NoiseGenerator_pink_octaves`, a pointer) that this does not carry.
If svara ever selects a non-white noise type, this must grow. State held in a
buffer rather than a scalar is the boundary that genuinely remains.

### Changed — corrections to what 3.3.0 and its docs claimed

The 3.3.0 CHANGELOG entry is corrected **in place** rather than quietly
superseded, because it stated the limitation as a measured finding. `ADR-0002`,
`state.md` and `tests/serde.tcyr`'s header are amended the same way.

**No naad change was needed.** The accessors already existed; nothing upstream
had to move.

### Notes

- The one restore limit that *is* real remains: **filter delay lines are not
  state**, and `tests/serde.tcyr` still asserts a warm tract and its restore
  differ. That matches the oracle — Rust's `set_formants` is
  `self.filter = FormantFilter::new(..)?`, which discards them on every call.
- Suite 826 → **828 assertions**, 20 suites.
- `dist/svara.cyr` regenerated at v3.3.2.

## [3.3.1] - 2026-08-26 — documentation, CI, and the claims that had gone false

**No source change.** A sweep of everything the repo asserts about itself. The
common thread: `cyrlint` reports *0 untracked deferrals* and always has, which is
accurate and nearly meaningless — it scans only `.cyr` sources, never Markdown,
matches twelve literal terms, and counts one "tracked" if the **same line**
contains `CHANGELOG`, `roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.`. That
is a substring test, not a check that the roadmap says anything. Everything below
was invisible to it.

### Fixed — CI ran almost none of the gate it documents

⭐ **`.github/workflows/ci.yml` had four steps: install, `cyrius deps`,
`cyrius build`, `cyrius test`.** No fmt, lint, docs, fuzz, deny, bench or
`cyrius audit` ran on any push or PR, while `CONTRIBUTING.md` names `cyrius audit`
as what a PR must pass and `state.md` documents eight gates. `release.yml`
inherited the same thin gate, so a release could ship having run only the tests.

That was defensible while `cyrius audit` was broken — it skipped dependency
resolution before compiling its test and bench legs and failed with spurious
*"undefined variable F64_ONE"* on any project with git-dep bundles. It was fixed
in 6.5.35 and svara has been on that pin since 3.1.2.

CI now runs `cyrius audit`, `cyrius fuzz` and `cyrius deny`, plus two new checks:

- **Toolchain matches the pin**, asserted *before* `cyrius deps` — the original
  failure mode surfaced two steps later as an opaque path error. `cyrius version`
  must equal the manifest pin and `~/.cyrius/versions/<pin>/lib` must exist.
  (hisab 2.11.2 added the same step after hitting exactly this.)
- **`dist/` is regenerable and current.** The bundle is what consumers actually
  compile and it is checked in, so it can drift from `src/` silently. CI
  regenerates and fails if the tree is dirty.

Both new steps were verified locally before being written, including that
`cyrius distlib` output is deterministic.

### Fixed — four documents that never left Rust

`README.md` and `CONTRIBUTING.md` were swept for Cyrius earlier; these four were
not, and still described a language this project does not use.

- **`docs/development/threat-model.md`** — the important one, and a prerequisite
  for the v1.0 security audit. It claimed `cargo deny` / `cargo audit` /
  "`deny.toml` restricts to crates.io only (no git dependencies)" for a project
  whose dependencies are now **entirely** git, and it asserted parameter
  validation mitigations that **3.1.3 disproved by finding five defects reachable
  through the public API**. Rewritten around what actually changed the model:
  Cyrius has no bounds checking on raw memory, `alloc` returns 0 rather than
  aborting, `f64_to` does not saturate, there are no unsigned types, and there is
  no panic. Resource exhaustion is now the sharpest section, because the bump
  allocator never frees.
- **`docs/architecture/overview.md`** — a `math` module that does not exist,
  "coefficients stored as f32 for processing" (the port is f64 throughout),
  compiler auto-vectorization (Cyrius emits scalar code), a `no_std` section, and
  "~5,000× real-time" against a measured **~40×**.
- **`docs/development/integration-guide.md`** — `use svara::prelude::*`,
  `vec![0.0f32; 512]`, and a Cargo feature table for features this library does
  not have. Rewritten around what a consumer actually needs: the manifest stanza,
  the fact that **svara's transitive dependencies resolve from the CONSUMER's
  manifest**, the error-code convention, and the zero-allocation streaming path.
- **`docs/guides/testing.md`** — `cargo test --all-features`, `make check`,
  criterion HTML reports. Rewritten around the real commands, plus the four rules
  a new assertion here has to follow, each traced to a defect that motivated it.

### Changed — the four pre-port ADRs are ADRs again

`docs/architecture/adr-001-source-filter-model.md` … `adr-004-scope-boundaries.md`
sat outside the scheme `docs/adr/README.md` states, were absent from its index,
and still referenced `bridge.rs`. They are **decisions**, so they moved to
`docs/adr/` as **0003–0006** and are indexed. Their substance is domain-level and
survives the port untouched; only Rust-specific wording was corrected.

`docs/architecture/README.md` had meanwhile declared itself *"Empty"* while
sitting beside all four.

### Changed — `.gitignore` no longer hides the benchmark history

⭐ **`*.csv` shadowed `benches/history.csv`**, so the file `docs/benchmarks.md`
describes as existing "so regressions are visible across commits" could never be
committed. 3.1.3 deliberately left this as "a call about committing a data file";
the call is made — the history is tracked.

Three more Rust-era rules retired with it, each with the reason recorded in the
file: `Cargo.lock` (`rust-old/Cargo.lock` is part of the frozen oracle and **is**
tracked, so the rule only created confusion), `**/*.rs.bk` (no rustfmt here) and
`/coverage/` (no coverage tooling here).

### Fixed — claims that had gone false

- **`CLAUDE.md`'s mission statement was still the scaffold placeholder** —
  `_TODO: one-or-two-sentence mission statement…_` — 3 releases and five months
  after the port. Written.
- **The bridge map count disagreed with itself**: README said 19, `state.md` and
  the CHANGELOG said 18. Counted — `src/bridge.cyr` defines **19** public
  `svara_bridge_*` functions. README was right; `state.md` is corrected.
- **`docs/benchmarks.md` listed 11 benchmark rows** where `benches/hotpath.bcyr`
  defines **13** — the two `render_planned` benches from 3.1.3 were missing. Added,
  with a note that they are the two rows a timer measures badly.
- **`docs/benchmarks-rust-v-cyrius.md`** was headed "v3.0.1 / cycc 6.4.11" and
  still called SIMD **P0** where the roadmap marks it deferred. It is now framed
  as the dated head-to-head it is — the figures stand as taken — with the
  priorities corrected: control-rate glides shipped, per-sample allocation
  (never on the list) mattered more than SIMD, and SIMD bought ~5% and was
  reverted.
- **README's consumer example pinned `tag = "3.1.2"`**, and its ADR link pointed
  at a file that just moved.
- **`docs/guides/getting-started.md`** sent new tests to `tests/svara.tcyr`, the
  smoke suite, rather than the per-module suite for whatever was touched.
- **The README / CONTRIBUTING Cyrius sweep shipped unrecorded** — it landed in a
  commit with no CHANGELOG entry, and `state.md` went on warning that both files
  "are still the pre-port Rust files" until 3.1.3. Recorded here.

### Notes

- `docs/guides/testing.md` now records **five coverage gaps** rather than leaving
  them implicit — `svara_tract_set_quality` appears in no suite or bench at all,
  so every quality-conditional branch runs Full-only under test. All five are
  scoped in the roadmap to be closed while `rust-old/` is still readable.
- The roadmap lane for this work was numbered 3.1.4 and was overtaken by 3.2.0 and
  3.3.0 shipping first; renumbered.
## [3.3.0] - 2026-08-26 — state companions: everything svara owns now round-trips

The follow-on ADR-0002 scoped. Nine engine types could not be derived — each
holds a nested struct pointer, a raw buffer, or a foreign naad handle — so each
now has a **flat state companion** the derive *can* reach, plus `save` / `restore`.
No hand-written codecs.

Derived types **13 → 22**; serde suite **54 → 135 assertions**; whole suite
**745 → 826** across 20 suites. `cyrius audit` exits 0, the 3.1.3 allocation
budget is intact, and every render path is still bit-for-bit identical to 3.1.2.

### Added — eight `save` / `restore` pairs and one direct derive

| Engine type | Companion | What restore rebuilds |
|---|---|---|
| `SvSpectrum` | *none needed* | already flat — `Vec<f64>` plus two scalars; it only wanted the annotations |
| `SvRenderOutput` | `SvRenderOutputState` | the nested `progress` pointer, inlined as three counters |
| `SvGlottalSource` | `SvGlottalState` | the naad noise generator + LFO; the Rng resumes exactly |
| `SvVocalTract` | `SvVocalTractState` | the formant bank and both naad biquads, from the recorded formants |
| `SvFormantKeypoint` | `SvKeypointState` | the nested `SvVowelTarget`, inlined as ten components |
| `SvTrajectoryPlanner` | `SvTrajectoryState` | the whole keypoint list, as `Vec<SvKeypointState>` |
| `SvSynthCtx` | `SvSynthCtxState` | the context, from the voice + the noise PRNG |
| `SvSynthesisPool` | `SvSynthesisPoolState` | as above, plus the two diagnostic counters |
| `SvBatchRenderer` | `SvBatchRendererState` | the event queue and the context |

⭐ **The context carries far less state than its field list suggests, and finding
that out is what made three of these small.** `svara_synthctx_synthesize` calls
`svara_tract_reset` and re-applies *every* glottal parameter from the supplied
voice at the top of each call — so the tract and glottal source hold nothing
across calls, and the buffer is scratch. What actually persists is the noise
PRNG and the sample rate. `SvSynthCtxState` is three fields.

**Restore takes the voice as an argument** for the context, pool and renderer. A
context is derived *from* a voice, and `SvVoiceProfile` already serializes on its
own; inlining its nine fields into three more structs would duplicate a schema
that then has to be kept in step.

### Changed — the tract now records its own formants and nasal place

Two pieces of information the tract used to throw away, both needed by restore
and both worth having anyway:

- **`formants`** — a built biquad bank holds coefficients, from which frequency
  and bandwidth cannot be recovered. The 3.1.3 `scratch_formants` vec is now that
  record as well as the scratch: `set_formants` syncs it after a successful
  rebuild. The sync copies **values**, is a no-op self-copy on the
  `set_formants_from_target` path, and allocates only when the formant *count*
  changes — so the per-sample allocation budget is unchanged, and
  `tests/allocbudget.tcyr` still passes.
- **`nasal_place`** — `set_nasal_place` used to retune the naad notch and forget
  which place it was. A tract could not report its own configuration. It is now
  stored and readable via `svara_tract_nasal_place`.

⚠ `SvVocalTract_scratch_formants` is now `SvVocalTract_formants`. Like 3.2.0's
`SvProsodyContour_xs`, it was `#derive(accessors)` output for internal scratch,
introduced two releases ago, never documented API.

### Fixed — a fresh tract misreported its own formants

Found by a control assertion, not by the feature test it belonged to. The first
version of the formant record left `svara_tract_new` setting it to a fresh zeroed
scratch vec while the filter was built from the schwa targets — so a
brand-new tract claimed **five 0 Hz formants**, `svara_tract_save` captured that,
and `svara_tract_restore` then correctly refused the state as invalid. The
constructor now keeps the vec it actually built the filter from.

The assertion that caught it was `"control: a well-formed state restores"` —
there to prove the *rejection* tests were not passing vacuously. It failed
instead, which is the control doing its job.

### Notes — what restore does NOT reproduce, measured rather than caveated

Two limits are real, and each is pinned by a test that asserts the divergence
*exists* — so they stay measured facts rather than warnings nobody checked.

- **naad's internal state cannot be read out.** A restored `GlottalSource`
  reproduces the deterministic pulse train **bit-for-bit over 1024 samples** when
  breathiness and vibrato are 0 — that is the whole output in that
  configuration. With breathiness > 0 the aspiration stream restarts, because
  naad's `NoiseGenerator` cell and `Lfo` phase are internal to naad. Asserted
  both ways: identical in the first case, *different* in the second.

  > ⚠ **Corrected in 3.3.2: this was wrong.** naad derives accessors on both
  > structs, so the state was reachable all along and the restore is now exact in
  > every configuration svara builds. The test that asserted the divergence now
  > asserts identity.
- **Filter delay lines are not state.** A restored `VocalTract` matches the
  original **over 512 samples from a clean filter state**, and a *warm* tract and
  its restore deliberately differ. That matches the oracle: Rust's `set_formants`
  is `self.filter = FormantFilter::new(..)?`, which discards the delay lines on
  every call, so there is nothing there a caller could have relied on.
  `lip_prev` and `interaction_feedback` — one-sample memories svara owns — *are*
  carried.

Behavioural equivalence is the standard throughout, as in 3.2.0: a restored
`SynthesisContext` renders the same noise-driven fricative (with a fresh context
asserted *not* to match, proving the stream really was resumed), a restored
`BatchRenderer` renders bit-identical audio, and a restored `TrajectoryPlanner`
interpolates identically across its span.

### Notes — restore validates, it does not trust

A state that has been through JSON is externally-authored data. Every `restore`
runs its inputs through the ordinary constructors: `svara_glottal_restore`
rejects an out-of-range or NaN `f0` and a non-positive `sample_rate`;
`svara_tract_restore` rejects a bad sample rate and an empty formant list. All
pinned, each alongside a control proving a well-formed state still restores.

- `dist/svara.cyr` regenerated at v3.3.0 (5,470 lines).
- **Why minor, not patch:** ten new public types and sixteen new public functions,
  plus the one accessor rename above.
## [3.2.0] - 2026-08-26 — container serde, and where serialization stops

**M2.** The serialized surface grows from 8 types to **13**, and — more usefully —
the boundary is now decided and written down instead of inherited. Suite
**713 → 745 assertions** across 20 suites. `cyrius audit` exits 0; every render
path stays bit-for-bit identical to 3.1.2.

### Added — five types now serialize

| Type | How |
|---|---|
| `SvSmoothedParam` | derived — it was already pure scalar and only needed three `f64` annotations |
| `SvRng` | derived — two `i64`; restoring a saved state **resumes the exact draw sequence** |
| `SvF0Point` | **new** — the raw 16-byte `(time, value)` pair given a type. Identical layout; `svara_pt_new`/`_time`/`_value` are unchanged in behaviour |
| `SvProsodyContour` | derived — `Vec<SvF0Point>` |
| `SvPhonemeSequence` | derived — `Vec<SvPhonemeEvent>`, whose element type is flat |

⭐ **Round-trips are asserted behaviourally, not field-by-field.** A restored
`Rng` must reproduce 32 draws exactly; a restored contour must trace the same
curve at 21 points; a restored `PhonemeSequence` must render **bit-identical
audio**. Equal fields are weaker evidence than equal behaviour, and the vacuous
round-trip is a live failure mode in this ecosystem — naad shipped one that held
for three releases because it compared through a truncating conversion.

### Changed — `ProsodyContour`'s interpolation scratch moved to module level

3.1.3 gave each contour its own `xs`/`ys`/`buf_cap` to get the allocation out of
the render loop. Those fields cannot coexist with serialization: **`#derive(Serialize)`
has no skip attribute**, so every field is emitted — a scratch pointer would leak
a heap address into the JSON (an ASLR disclosure if it crosses a trust boundary)
and come back dangling.

The buffers hold nothing between calls — they are refilled on every one — so a
single module-level pair serves every contour. That is less memory than the
per-contour pair, keeps the raw pointers out of the serialized surface, and the
allocation budget is unchanged.

⚠ **`SvProsodyContour_xs` / `_ys` / `_buf_cap` are gone.** They were
`#derive(accessors)` output for internal scratch, existed only in 3.1.3, and were
never documented API.

### Fixed — the roadmap told M2 to match something that does not exist

M2 instructed an implementer to *"match `rust-old/`'s `#[derive(Serialize,
Deserialize)]` / `#[serde(skip)]` per type"*. Transcribed from the oracle, the
contract is **30 `Serialize`-deriving types, 3 `#[serde(default)]`, and zero
`#[serde(skip)]`**. There was no skip policy to inherit.

Rust could serialize everything because `VocalTract` holds a
`naad::filter::BiquadFilter` (`rust-old/src/tract.rs:72`, `:94`) and that type is
`Serialize` in the Rust naad. Cyrius naad exposes those filters as opaque handles
with no JSON codec, so the policy had to be **decided**, not transcribed. It now
is — [ADR-0002](docs/adr/0002-serialization-boundary.md).

### Notes — three toolchain facts, measured

Verified with a standalone probe on cyrius 6.5.35, not read off a changelog:

- `Vec<f64>` / `Vec<iNN>` fields round-trip. ✅
- `Vec<T>` where T is a **flat** `#derive` struct round-trips — via
  `_from_json_str` **only**. The pairs-form `_from_json` returns an *empty vec*
  for an array-of-objects field, because bayan truncates that value at the first
  inner comma. Silent, so it is stated in each affected module header.
- ⚠ **A nested `#derive`-struct field is broken in both directions, and worse
  than "unsupported".** It is laid out **inline** — `sizeof(ND{tag: i64; inner: NA})`
  is 24 for a 16-byte `NA` — while `#derive(accessors)` gives the same field
  **pointer** semantics. The two derives disagree, so the encoder emits the
  pointer reinterpreted as the first member: `{"inner":{"x":6.9e-310,"n":0}}`
  where the value was `{x: 99, n: 3}`. Written inline instead, the encoder is
  correct. **Either way the decoder returns zeros**, so a decoded object holds a
  null pointer where a struct should be — the next dereference is a segfault, the
  same shape as ADR-0001's findings.

svara is not exposed to that corruption: every struct-valued field in `src/` is
declared untyped (`filter;`, `progress;`, `target;`) and so is a plain slot with
correct pointer semantics. **Adding a type annotation to make `Serialize` see
such a field would change its layout and break the accessors** — a trap now named
in ADR-0002.

### Not shipped, and why

`VocalTract`, `GlottalSource`, `FormantFilter` / the SOA bank,
`TrajectoryPlanner`, `SynthesisContext`, `SynthesisPool`, `BatchRenderer`,
`RenderOutput` and `Spectrum` are **not** derived. Each holds a nested struct
pointer, a raw sample buffer, or a foreign naad handle with no JSON form at all.
Under ADR-0002 these are engine state, not configuration: a consumer rebuilds
them from the configuration that does serialize.

`tests/serde.tcyr`'s final group pins the two facts that make them un-derivable —
`FormantKeypoint` holds a target pointer, `GlottalSource` holds three dependency
handles — so a toolchain release that fixes nested-field decode shows up here as
a **failing test** rather than going unnoticed.

Giving each of them a flat state companion plus a `restore` is the mechanical
follow-on, scoped in the roadmap.

### Notes

- **The three `#[serde(default)]` behaviours are not reproduced** — Cyrius's
  derive has no default mechanism and a missing field decodes to zero. For
  `PhonemeEvent.tone` that is wrong in a specific way: `0` is `SVARA_TONE_HIGH`,
  while the Rust default meant "no tone", which svara spells
  `SVARA_TONE_NONE = -1`. svara always emits the field, so this only bites on
  externally-authored JSON — worth a guard if that becomes a supported input.
- **The 13 Rust `Serialize` enums need no counterpart.** They are `var SVARA_*`
  integers here and already round-trip wherever they appear as a field
  (`SvNasalization.place`, `SvPhonemeEvent.stress`/`.tone`). Deriving a one-field
  wrapper per enum would add API surface to serialize an integer.
- `dist/svara.cyr` regenerated at v3.2.0 (4,974 lines).
## [3.1.3] - 2026-08-26 — P1 audit: five reachable defects, and the leak in the render loop

An adversarial-input and allocation sweep of the whole public surface. **Five
defects reachable through the public API with no unsafe usage by the caller** —
one SIGSEGV, three process aborts, one silent NaN — plus the per-sample
allocation that made the trajectory-planned render path scale in gigabytes.

Suite **634 → 713 assertions** across 20 suites. `cyrius audit` exits 0; fuzz and
deny clean. **Every render path is bit-for-bit identical to 3.1.2** — ten
representative renders, digest-compared across the two trees.

### Fixed — SIGSEGV: `alloc` failure was never checked

`lib/alloc.cyr`'s `alloc` **returns 0** on failure (`size <= 0`, `size >
ALLOC_MAX` = 2 GiB, or mmap refusal). It does not abort, the way Rust's
allocator does — so the Rust code had nothing to check and the port carried no
check over. Every size-driven allocation in svara wrote through the returned
pointer on the very next line.

⭐ **`svara_phoneme_synthesize(VowelA, voice, 44100.0, 6100.0)` segfaulted.**
6100 seconds is 269,010,000 samples → 2,152,080,000 bytes, just past `ALLOC_MAX`;
`alloc` returned 0 and `svara_formant_zero_buf` then stored to address 0 and
upward. Confirmed as signal 11, not inferred. It now returns
`SVARA_ERR_COMPUTATION`.

- New `svara_alloc_samples(n)` in `src/error.cyr` — bounds `n` against
  `SV_MAX_SAMPLES` **before** the `* 8`, so the product cannot wrap past its own
  cap, and returns 0 for anything unusable. **19 allocation sites** — 12 in
  `phoneme.cyr`, 3 in `spectral.cyr`, 2 in `prosody.cyr`, 1 each in
  `sequence.cyr` and `tract.cyr` — now route through it and check the result.
- `svara_tract_synthesize` returns 0 rather than a buffer it could not allocate;
  its three callers check.
- `svara_synthctx_ensure_buffer` no longer stores a null over a working buffer on
  a failed grow — that would have poisoned every later call on the context.

### Fixed — `f64_to` does not saturate, and the sentinel reached sample counts

Rust's float → int `as` casts saturate (NaN → 0, negative → 0, overflow → MAX).
Cyrius `f64_to` returns **`INT64_MIN`** for NaN, `±inf` and anything outside
`i64` — measured on this toolchain, not assumed. So the ported line
`let n = (duration * sample_rate) as usize;` yielded `INT64_MIN` wherever Rust
yielded 0 or a cap.

- New `svara_count_from_f64(x)` reproduces Rust's saturating `as usize`.
  **Eleven cast sites** now use it (sample counts, sample offsets, nasalization
  onsets, spectrum bin indices, the crossfade floor, the pool pre-warm).
- ⭐ **A NaN sample rate now returns an empty vec, as Rust does.** Rust checks
  only `sample_rate <= 0.0` there, which is false for NaN, so the cast saturates
  to 0 and `synthesize_phoneme` returns `Ok(Vec::new())`. svara returned
  `INT64_MIN` and carried on. Parity restored, and pinned.
- Three raw `f64_to` calls remain, all provably bounded (the VOT fractions are
  in-tree constants, the crossfade fraction is built from in-tree coarticulation
  constants). Each now says so at the call site.

### Fixed — process abort: a negative spectrum bin reached `vec_get`

Rust's `Spectrum::frequency_bin` returns `usize`, so a negative or NaN frequency
— or a zero `freq_resolution`, which makes the quotient `±inf` — saturates to 0.
The three callers can then test only the *upper* bound, and Rust's do. Ported to
signed `i64` that is half a bound, and `f64_to`'s `INT64_MIN` went straight
through it into `vec_get`, which aborts.

⭐ `svara_spectrum_magnitude_at`, `svara_spectrum_band_energy` and
`svara_spectrum_peak_in_range` **killed the process** on any negative or NaN
frequency argument. Fixed at the source: `svara_spectrum_frequency_bin` now
saturates, so all three callers' single bound is a complete bound again.

### Fixed — silent NaN: an inverted range test accepted a NaN f0

Rust writes `if !(20.0..=2000.0).contains(&f0) { reject }`. `contains` is
`20.0 <= f0 && f0 <= 2000.0`, **false for NaN** — so Rust *rejects* a NaN f0.
The port split it into two one-sided rejections (`f0 < 20`, `f0 > 2000`), and
`f64_lt`/`f64_gt` are both false for NaN, which inverts the meaning.

⭐ `svara_glottal_new` and `svara_glottal_set_f0` **accepted NaN**, stored it,
returned `SVARA_ERR_NONE`, and every sample derived from it came out NaN. Both
now go through `svara_f0_in_range`, which tests the range positively so NaN falls
through to the reject.

### Fixed — a constructor that reported success by handing back an error code

`svara_tract_new` builds a formant filter, retries with a single-500 Hz fallback,
and — before this release — stored whatever came back in the `filter` field. On a
non-positive or non-finite sample rate **both attempts fail**, so it stored the
negative error code as a pointer and returned a tract that looked valid; the
first `process_sample` dereferenced a small negative address.

Rust's `VocalTract::new` returns `Self` and ends that fallback with
`.expect("fallback formant filter must succeed")` — it panics. The port had
neither the panic nor a check. `svara_tract_new` now returns the error, and all
**13 call sites** check it.

### Fixed — infinite loop in `svara_next_pow2`

`p = p * 2` wraps negative past 2^62 and `p < n` then stays true forever, so an
input above that hung the process. It now reports 0 and `svara_spectral_analyze`
rejects it.

### Changed — the render loop stopped allocating per sample

Cyrius's bump allocator **never frees**, so an allocation inside a per-sample
loop is not a performance nuisance, it is an unbounded leak.

⭐ **`svara_sequence_render_planned` spent 1,121 bytes of arena per output sample
untoned and 1,505 toned** — measured as the marginal cost of doubling the
duration, so the fixed setup cancels. A minute of speech at 44.1 kHz therefore
needed ~3 GB (untoned) or ~4 GB (toned) that was never released.

| | 3.1.2 | 3.1.3 | |
|---|---|---|---|
| untoned | 1,121 B/sample | **33 B/sample** | 34× |
| toned | 1,505 B/sample | **113 B/sample** | 13× |

What remains is the output buffer plus the vec it is copied into (8 bytes each
per sample); the per-sample synthesis work itself now allocates nothing. The
toned path additionally pays ~80 B/sample to hisab's `calc_monotone_cubic`,
which allocates three internal arrays per call — upstream, not svara's.

Where it went, per sample, measured call by call:

- **`svara_tract_set_formants`: 736 B → 0.** It called `svara_formant_filter_new`
  and discarded the previous filter. New `svara_formant_filter_rebuild` puts the
  filter into *exactly* the state `new` would have produced — every slot zeroed,
  delay lines cleared, DC blocker reset — while reusing the storage. That
  discard-the-state behaviour is Rust's (`self.filter = FormantFilter::new(..)?`)
  and is preserved deliberately, not "fixed"; only the allocation is saved.
  Validation runs before any mutation, so a rejected call leaves the filter
  untouched, as Rust's `?` does.
- **`svara_vowel_target_to_formants`: 272 B → 0.** New
  `svara_vowel_target_to_formants_into` overwrites five reusable `SvFormant`
  structs owned by the tract.
- **`svara_trajectory_formants_at`: 80–240 B → 0.** New
  `svara_trajectory_formants_at_into`, with the two Catmull-Rom intermediates in
  planner-owned scratch targets.
- **`svara_tone_to_contour`: 240 B → 0.** It was rebuilt every sample from a
  value — the event's tone — that only changes at a phoneme boundary. Now built
  once per event, before the loop.
- **`svara_prosody_f0_at`: 144 B → 80 B.** The two f64 buffers hisab needs are
  now contour-owned. They are **refilled on every call, never cached**, so a
  caller mutating a point through `f0_points` cannot read a stale value; only the
  allocation is reused. The residual 80 B is inside `calc_monotone_cubic`.
- **`svara_formant_bank_update`: 32 B → 0.** The coefficient scratch is
  bank-owned.

CPU time improved too, though that was not the point: **render_planned
20.638 → 18.598 ms** (−10%) and **toned 25.271 → 21.520 ms** (−15%) for the same
three-phoneme sequence.

### Added — two suites, and the ADR that names the class

- **`tests/hardening.tcyr` (45 assertions)** — every defect above, pinned.
  ⚠ **Discrimination measured, not asserted.** Ten of its probes were re-run
  against a 3.1.2 checkout on the same toolchain: **9 of 10 fail there** — four
  by wrong value, three by process abort (`vec: index < 0`), one by SIGSEGV, one
  by hang — and the tenth is the control, which passes on both sides. A group
  where every line flipped would mean the controls were not controls.
- **`tests/allocbudget.tcyr` (34 assertions)** — pins the two properties the
  allocation work rests on. Each `_into` variant is compared **bit for bit**
  against the allocating function it replaced (exact `assert_eq` on raw f64
  patterns, not tolerances — the two paths run the same arithmetic in the same
  order, so this holds on every architecture, unlike a stored golden digest).
  The budget itself is asserted as marginal bytes per sample, each bound paired
  with a control proving the measurement is not reading zero.
- **[ADR-0001](docs/adr/0001-signed-index-and-float-conversion-hazards.md)** —
  the hazard class, its four mechanisms, and the five standing conventions.
  Written because goonj recorded the same two root causes independently, and
  because its 2.0.2 release swept two modules for exactly this and missed a
  third: an unnamed class does not get swept completely.

### Notes

- **Bit-for-bit parity verified directly.** Ten renders — vowel, diphthong,
  fricative, plosive, nasal, crossfade sequence, planned sequence untoned and
  toned, nasalized vowel, female-voice diphthong — were FNV-1a digested over
  their raw f64 sample patterns and compared between a 3.1.2 tree and this one.
  All ten digests match, and the lengths match.
- **Two `benchmarks` added** for `render_planned` (untoned and toned), a major
  public path that had no coverage. The allocation budget is *not* benchmarked —
  a bump allocation is nearly free in CPU time and would not show up — it is
  pinned as a test instead.
- **Preserved deliberately, not overlooked**: `svara_formant_validate` accepts a
  NaN formant frequency, because Rust's
  `f.frequency <= 0.0 || f.frequency >= nyquist` accepts it too. Annotated in
  `src/formant.cyr` rather than quietly tightened — the parity bar is "matches
  what Rust did", and diverging needs an ADR.
- **Known residual**: `svara_ph_buf_to_vec` builds its result with `vec_push`
  from empty, so the vec's doubling leaves roughly one dead copy of the output in
  the arena. Pre-sizing it would mean reaching into `lib/vec.cyr`'s header
  layout; the right fix is a `vec_with_capacity` upstream.
## [3.1.2] - 2026-08-26 — toolchain + dependency catch-up

Maintenance release. Bumps the Cyrius pin **6.4.13 → 6.5.35** (109 toolchain
releases) and every pinned dependency to its current tag. One **breaking upstream
rename** had to be absorbed; behaviour is otherwise unchanged. Suite **634
assertions / 18 suites**, all green; fmt · lint · docs · fuzz · deny clean, and
`cyrius audit` now exits 0 (see below).

### Changed — toolchain

- **`[package].cyrius` `6.4.13` → `6.5.35`.**
- **`lib/` re-vendored from the 6.5.35 snapshot** — all 29 stdlib files verified
  byte-identical to `~/.cyrius/versions/6.5.35/lib`, file by file, rather than
  assumed. `lib/callback.cyr` is newly vendored (a stdlib leaf the refreshed
  dependency bundles now fold in).
- **90 lines reindented across 12 files.** cyrius **6.5.28** fixed `cyrfmt`,
  which had never tracked parentheses: continuation lines inside an unclosed `(`
  were indented at `brace_depth * 4` regardless of nesting. Canonical is now
  2 spaces per open-paren level. **Whitespace only** — `git diff -w` over
  `src/`, `tests/` and `benches/` is empty apart from the `NAAD_FILTER_*` rename
  below.

### Changed — dependencies

| Dep | Was | Now | |
|---|---|---|---|
| hisab | 2.6.7 | **2.11.2** | FFT / compensated sum / monotone-cubic / easing |
| naad | 2.1.1 | **2.2.1** | biquad · noise · LFO backends |
| goonj | 2.0.0 | **2.0.4** | acoustics, referenced by the naad bundle |
| sakshi | 2.4.2 | **2.4.11** | transitive via hisab |

All four match what the bundles themselves pin — naad 2.2.1 pins hisab 2.11.2
and goonj 2.0.4; hisab 2.11.2 pins sakshi 2.4.11 — so a downstream consumer
(dhvani / vansh) bundling svara + naad + hisab in one flat namespace gets a
coherent set.

### Fixed — BREAKING upstream rename absorbed (`src/tract.cyr`)

naad **2.2.0** renamed all eight `FILTER_*` constants to `NAAD_FILTER_*` to clear
a flat-namespace collision with `nidhi`, which defines `FILTER_LOWPASS..NOTCH` at
**identical values**. svara calls exactly two of them:

- `FILTER_NOTCH` → `NAAD_FILTER_NOTCH` (nasal antiformant)
- `FILTER_BANDPASS` → `NAAD_FILTER_BANDPASS` (subglottal bandpass)

Values are unchanged (3 and 2), so this is a compile-time break, not a behaviour
change — the tract suite's 14 assertions pass unchanged.

⚠ **The other renames in that wave were checked and none reach svara.** The full
set of dependency symbols svara actually references was extracted — 24 candidate
identifiers, 17 real top-level references plus the derive-generated
`Lfo_set_depth` accessor — and diffed against the new bundles: `ERR_* → NAAD_ERR_*`
(2.1.3) is inert here because svara's codes are `SVARA_ERR_*`-prefixed and its one
bare `ERR_NONE` mention is a comment; `VOICE_*`, `lerp`, `rms`, `peak`,
`crossfade_equal_power` and the removed `white_noise_sample` / `U32_MAXF` have no
call site. Every remaining signature — `num_fft`, `num_neumaier_sum`,
`calc_monotone_cubic`, `ease_in_out_smooth`, the four `filter_biquad_*`, the two
`noise_*`, the three `modulation_lfo_*` — is byte-identical across the bump.

### Fixed — a stale constant in the benchmark harness

`benches/hotpath.bcyr` and `docs/benchmarks.md` both stated per-call clock
overhead as **"~240 ns on this host"**. cyrius 6.5.19 taught `lib/bench.cyr` to
**measure** the timer floor and subtract it from every sample, and upstream
retired the figure outright: a clock read spans ~15 ns (macOS arm64) to ~3,550 ns
(aarch64 Linux), a 230× range that no single number in a comment can carry. The
harness now prints its own measured floor — **1.34 µs on this host, 5.6× the
number that was written down** — and `bench_clock_overhead_ns()` returns it. Both
mentions now point at the function instead of quoting a value.

The batch-timed figures themselves did not move (`bench_run_batch1/2` already
wrapped one clock pair around 1000 calls), which is what makes the stale comment
harmless in effect and worth fixing anyway.

### Changed — `cyrius audit` is now the gate again

3.0.0 recorded that `cyrius audit` **skipped dependency resolution** before
compiling its test and bench legs, so both failed with spurious *"undefined
variable F64_ONE / SYS_WRITE"* on any project whose tests need the stdlib prelude
plus git-dep bundles — reproducible identically in naad 2.1.0, hence a toolchain
bug rather than a svara defect. The workaround was to gate on the individual
tools.

On 6.5.35 `cyrius audit` resolves deps first (`4 deps resolved`) and **exits 0**:
fmt clean · lint clean · docs complete · 18 suites / 634 assertions · 2 bench
files. The per-tool commands still work as the finer-grained form; the aggregate
is simply no longer broken.

### Notes

- **Benchmarks re-run, no regression.** Per-sample: glottal 82 ns, formant
  179 ns, tract 295 ns. Renders: `/a/` 857 µs, `/s/` 458 µs, `/ai/` 921 µs,
  3-phoneme sequence 3.36 ms. Every row is within a few percent of the 3.1.0 run
  — the spread is host noise, not the toolchain. Recorded in
  `benches/history.csv`; `docs/benchmarks.md` refreshed (its results table still
  carried the pre-3.1.0 `/ai/` figure of ~5.4 ms).
- **`dist/svara.cyr` regenerated at v3.1.2** (4,543 lines). The `.deps` sidecar
  grew from 2 entries to 16 — `cyrius distlib` now emits the full stdlib leaf set
  the bundle folds in, not just the opt-in `hashmap` / `bayan` pair.
- **The 3.1.0 entry's assertion count was wrong and is corrected in place.** It
  said 652; `src/` and `tests/` are byte-identical between the 3.1.0 tag and 3.1.1
  and the suite measures **634** on both, so 652 was never right. `state.md`,
  which said 634, was the one telling the truth.
- **`docs/development/dependency-watch.md` rewritten.** It was still the pre-port
  Rust table — hisab 1.2, naad 1.0, libm, serde, thiserror, tracing, criterion,
  `cargo audit` / `cargo deny`, and a `codecov.yml` coverage threshold for a file
  that does not exist in this repo. It now carries the real pins, the symbols
  svara consumes from each, the flat-namespace rename history, and the procedure
  for checking the next bump.
- **M2 (container serde) is no longer blocked.** It was gated on array-typed
  struct fields, which landed across cyrius **6.4.11–6.4.13** (`Vec<T>` handle
  fields → `#derive` for `Vec<primitive>` → `Vec<#derive-struct>`). Deliberately
  **not** started here — this release is a pin bump, and M2 is its own milestone.

### Known — not addressed here

- **`README.md` and `CONTRIBUTING.md` are still the pre-port Rust files.** They
  survived `cyrius port` untouched: "Formant and vocal synthesis for **Rust**",
  crates.io links for hisab / naad, a `use svara::prelude::*` example, a Cargo
  feature-flag table, "48 phonemes" where the port ships 101, "~1,000× real-time"
  where the measured figure is ~40×, and a quality-gate section made entirely of
  `cargo` commands. Out of scope for a pin bump; they need their own doc sweep.
- **`benches/history.csv` cannot be committed.** `.gitignore` carries a Rust-era
  `*.csv` rule (almost certainly meant for coverage output), so the benchmark
  history is untracked — which defeats the "so regressions are visible across
  commits" purpose `docs/benchmarks.md` states for it. Left alone because
  un-ignoring it is a call about committing a data file, not a maintenance fix.

## [3.1.1] - 2026-08-26 — toolchain pin (retroactive entry)

Pin-only release, tagged without a changelog section at the time; recorded here so
the release workflow's changelog extraction has something to publish.

### Changed

- **`[package].cyrius` `6.4.12` → `6.4.13`.** No source, dependency or behaviour
  change — `VERSION` and the manifest pin are the entire diff.

## [3.1.0] - 2026-07-06 — synthesis performance (control-rate glides)

Performance minor. The faithful v3.0.x port re-derived formant filter coefficients
on every sample of a vowel glide; this release moves that to control rate.

### Changed
- **Diphthong synthesis ~5.8× faster** (`phoneme synth /ai/`: 5.42 ms → 0.94 ms).
  `svara_ph_synth_diphthong` re-solved the entire formant biquad bank from the
  interpolated target on *every* sample; it now recomputes at a control rate of 64
  samples (~1.45 ms at 44.1 kHz — standard for formant synthesizers) and holds the
  coefficients between updates. The per-sample tract filtering is unchanged.
  Perceptually identical (the glide target moves smoothly); the 634-assertion
  tolerance suite passes unchanged. [*Corrected 3.1.2: this entry originally said
  652. `src/` and `tests/` are byte-identical between the 3.1.0 tag and 3.1.1, and
  the suite measures 634 on both, so 652 was never right.*] The diphthong path now matches a steady vowel
  (~0.86 ms) and is faster than the Rust oracle (1.09 ms), which still re-solves
  per sample.
- **Toolchain pin 6.3.40 → 6.4.12** (current release; removes drift, aligns with
  downstream consumers).

### Notes
- SIMD vectorization of the formant biquad bank (`f64v4`) was prototyped and
  reverted: the per-sample loop is **memory-bound** (the SOA state shuffle +
  builtin-call overhead dominate), so packed arithmetic bought only ~5% while the
  ptr/value vector-op API can't keep lane state in registers across samples. The
  real bit-identical lever there is eliminating the redundant per-slot input delay
  line (x1/x2 are identical across all 8 formant slots — the input is shared),
  tracked as future M-perf work alongside register-resident SIMD.

## [3.0.1] - 2026-07-05 — naad 2.1.1 (namespace de-collision)

Dependency maintenance. Bumps the pinned naad dependency **2.1.0 → 2.1.1**, in
which naad renames its two bare dB helpers (`amplitude_to_db` / `db_to_amplitude`
→ `naad_amplitude_to_db` / `naad_db_to_amplitude`) to clear a flat-distlib symbol
clash with abaco. svara does **not** call those helpers — it consumes naad's
biquad / noise (`NoiseGenerator`) / LFO (modulation) backends — so this is a
pin-only bump with **no svara source or behavior change**. It keeps svara
coherent with naad 2.1.1 for downstream consumers (dhvani / vansh) that bundle
svara + naad (+ abaco) in one namespace.

### Changed

- `[deps.naad]` tag `2.1.0` → `2.1.1`. Re-vendor `lib/naad.cyr` with `cyrius deps`
  once the naad 2.1.1 tag is published.

## [3.0.0] - 2026-07-03 — Cyrius port (full parity)

Complete Rust → Cyrius port: **full behavioral parity** with the Rust 2.0.0
surface. All 19 Rust modules ported (16 `.cyr` modules), **634 test assertions
across 18 suites** (vs the Rust 213 tests), **11 hot-path benchmarks** (covering
the intent of the 15 Rust criterion benches), fmt/lint/docs all clean. The Rust
source is preserved at `rust-old/` as the parity oracle. Port plan and locked
decisions: [`docs/development/state.md`](docs/development/state.md),
sequencing: [`docs/development/roadmap.md`](docs/development/roadmap.md).

### Added (port scaffold + foundation layer)

- **Scaffold** via `cyrius port`: `cyrius.cyml` manifest (stdlib incl. `math`,
  `ganita`, `tagged`, `bench`; `[lib]` bundle list), `src/main.cyr` smoke entry,
  `.tcyr`/`.bcyr`/`.fcyr` harnesses. Toolchain pinned to `6.3.38`.
- **`src/error.cyr`** (L0) — ports `error.rs` (`SvaraError` → integer `SVARA_ERR_*`
  codes) + `dsp.rs` validators (`svara_validate_sample_rate`/`_duration`) + shared
  f64 tolerances (`SV_EPSILON`, `SV_POS_INF`) and `svara_is_finite`. **25 tests.**
- **`src/rng.cyr`** (L0) — ports `rng.rs` PCG32 (`SvRng`, seedable). u64/u32 math
  hand-emulated on i64 (`& 0xFFFFFFFF` masking, logical shifts). Verified against
  golden values from a faithful algorithm replica. **17 tests.**
- **`src/smooth.cyr`** (L0) — ports `smooth.rs` one-pole `SvSmoothedParam`
  (`exp(-1/(τ·sr))` via ganita builtin). **6 tests.**
- **`src/formant.cyr`** (L1) — ports `formant.rs`: `Formant`, `Vowel`,
  `VowelTarget` (Hillenbrand 1995 table, interpolation) + the structure-of-arrays
  parallel bandpass biquad bank (`MAX_FORMANTS=8`), `DcBlocker`, and
  `FormantFilter`. svara's own SOA topology (not naad's `BiquadFilter`); nine
  `[f32;8]` arrays become raw f64 buffers held as struct fields. Golden biquad
  coefficients + full impulse-response output verified against an f64 oracle.
  **25 tests.** No external dep.
- **`src/spectral.cyr`** (L1) — ports `spectral.rs`: `Spectrum` + FFT `analyze`
  (Hann window, hisab `num_fft` over an interleaved HComplex buffer), `rms_level`,
  `total_energy`/`band_energy` (hisab `num_neumaier_sum`), peak picking and
  `estimate_formants`. **14 tests.** Adds the **hisab** git dep (pinned `2.6.7`).
- **`src/glottal.cyr`** (L2) — ports `glottal.rs`: `GlottalSource` with Rosenberg B,
  LF (Rd-parameterized via `LfParams::from_rd`), Whisper, and Creaky models, plus
  jitter/shimmer/breathiness/diplophonia/vibrato. Carries the **naad-backend path**
  (aspiration noise = naad `NoiseGenerator::White`, vibrato = naad `Lfo::Sine`;
  svara's PCG32 still drives jitter/shimmer/period). Golden-verified LF params and
  first-period Rosenberg output. **35 tests.** Adds the **naad** (`2.1.0`) + **goonj**
  (`2.0.0`) git deps (full naad bundle).
- **`src/lod.cyr`** (L4, pulled early) — ports `lod.rs` `Quality` (Full/Reduced/Minimal)
  + pipeline-stage predicates (`max_formants`, `use_nasal_coupling`, `use_subglottal`,
  `use_interaction`, `use_lip_radiation`). **15 tests.**
- **`src/tract.cyr`** (L2) — ports `tract.rs`: `VocalTract` + `NasalPlace`. Pipeline =
  source-filter interaction feedback → `FormantFilter` → nasal antiformant (naad `Notch`
  biquad, place-dependent) → subglottal resonance (naad `BandPass` ~600 Hz) → lip-radiation
  HPF → gain, all LOD-gated. Carries the naad-backend path (CFG split collapsed).
  Determinism verified. **14 tests.**
- **`src/phoneme.cyr` (part 1/3)** (L3) — ports `phoneme.rs` inventory +
  classification: the 101-phoneme IPA inventory (`Phoneme` → integer constants in
  exact Rust order), `PhonemeClass`, and the pure lookups `class` / `is_voiced` /
  `coarticulation_resistance` (Recasens DAC). All 101 phonemes golden-verified
  against an independent transcription of the Rust match logic. **229 tests.**
  Data tables (part 2) + synthesis (part 3, after `voice`) follow. The phoneme
  tables are `match`-based pure functions → ported as embedded Cyrius (if-chains),
  not externalized data files.
- **`src/phoneme.cyr` (part 2/3)** (L3) — the data tables: `phoneme_formants`
  (full 101-arm locus/steady-state table), `phoneme_duration`, `phoneme_spectral_tilt`,
  `height_adjusted_amplitudes`, `f2_locus_equation`, `VoiceOnsetTime` (+`for_plosive`),
  `Nasalization` (+`for_nasal`), `detect_nasalization`. Integer targets exact;
  computed values (tilt, height amps) golden-verified. **+44 tests (273 total).**
- **`src/voice.cyr`** (L3) — ports `voice.rs`: `VoiceProfile` (presets
  male/female/child, builder methods, `apply_formant_scale` with f0/singing
  bandwidth scaling), `VocalEffort` + `EffortParams` (whisper..shout continuum →
  coordinated glottal params), and `create_glottal_source[_with_effort]`.
  Formant-scaling goldens + effort energy ordering verified. **33 tests.**
- Cleanup: all 10 `.tcyr` suites brought under the 120-col lint (test files are
  audit-gated too).
- **`src/phoneme.cyr` (part 3a/3)** (L3) — the free synthesis path:
  `synthesize_phoneme` + `synthesize_phoneme_nasalized` dispatching to per-class
  synthesizers (vowel, diphthong, plosive, fricative, nasal, approximant,
  affricate, trill, click, ejective, implosive), plus `fricative_formants`,
  `diphthong_end_target`, and the attack/release envelope (hisab
  `ease_in_out_smooth`). Ties voice + glottal + tract + `FormantFilter` into
  audio (returns a vec of f64 samples). **+17 tests (290 total).** svara can now
  synthesize a phoneme end-to-end. `SynthesisContext` (part 3b) follows.
- **`src/phoneme.cyr` (part 3b/3)** (L3) — `SynthesisContext`: the allocation-reuse
  wrapper owning a `VocalTract` + `GlottalSource` + shared PRNG + growable scratch
  buffer, resetting state between phonemes (the path multi-phoneme rendering drives).
  Ports `synthesize` + the per-class `synthesize_*_ctx` methods (which use fixed
  n/3,n/6,n/4,n/8 timing, not VOT, and route Trill through the approximant path,
  faithful to the Rust). **+9 tests (299 total).** **phoneme is now fully ported**
  (the crate's largest module, 2,636 LOC).
- **`src/prosody.cyr`** (L3) — ports `prosody.rs`: `ProsodyContour` (f0 point list
  interpolated with hisab monotone-cubic; `apply_stress` with dynamic f0-boost
  insertion + sort), the 4 `IntonationPattern`s, 9 lexical `Tone`s (`to_contour`),
  and `Stress`. Contour constants use Cyrius decimal float literals. **17 tests.**
- Toolchain pin → `6.3.39`.
- **`src/trajectory.cyr`** (L3) — ports `trajectory.rs`: `TrajectoryPlanner` +
  `FormantKeypoint`. `plan` builds lead/midpoint/trailing keypoints from phoneme
  events; `formants_at` interpolates with 4-point Catmull-Rom (resistance-weighted
  blend with linear) or sigmoid fallback; `apply_speaking_rate` (Lindblom
  undershoot). Endpoint/boundary/finite behavior verified. **12 tests.**
- **`src/sequence.cyr`** (L3) — ports `sequence.rs`: `PhonemeEvent` / `PhonemeSequence`.
  `render` (per-phoneme synthesis with stress/cluster duration scaling + anticipatory
  nasalization, then resistance-weighted variable crossfade), `render_planned`
  (trajectory-driven continuous synthesis with per-sample formant updates + tone f0),
  `detect_consonant_clusters`, and the variable crossfade blender. **29 tests.**
  Completes the L3 speech-science layer — svara renders whole utterances.
- **`src/pool.cyr`** (L4) — ports `pool.rs`: `SynthesisPool` (pooled `SynthesisContext`
  with `render`/`render_nasalized`/`render_batch`, pre-warmed `with_capacity`, and
  render-count/peak-samples diagnostics). **16 tests.**
- **`src/render.cyr`** (L4) — ports `render.rs`: `BatchRenderer` / `RenderOutput` /
  `RenderProgress` (queue phonemes, render concatenated, stress-scaled + nasalized).
  The Rust FnMut progress callback becomes a `callptr`-based fn-pointer + user-data
  cell. **16 tests.**
- **`src/bridge.cyr`** (L4) — ports `bridge.rs`: 18 dependency-free scalar bridges
  mapping upstream AGNOS crate outputs (bhava emotion, vansh TTS, prani creature,
  goonj acoustics, badal weather) to synth parameters. **37 tests.**
- **All 19 Rust modules are now ported** (16 `.cyr` modules; `dsp` folded into
  `error`, `math` maps to `ganita`). **610 tests total, all lint-clean.**
- **Serialization** — 8 pure-scalar public value types use Cyrius `#derive(Serialize)`
  (`Type_to_json`/`_from_json_str`, f64 fields via toolchain v6.3.40): `Formant`,
  `VowelTarget`, `VoiceProfile`, `EffortParams`, `PhonemeEvent`, `Nasalization`,
  `VoiceOnsetTime`, `RenderProgress`. JSON round-trip tests in `tests/serde.tcyr`
  (**22 tests → 632 total**). Container types (vec/buffer-bearing) await Cyrius
  array-typed struct fields (v6.4.x). Toolchain pin → `6.3.40`.
- **distlib** — `cyrius distlib` bundles all modules into `dist/svara.cyr` (+ `.deps`)
  for consumers (dhvani/vansh), which supply hisab/naad/goonj + stdlib.
- **Benchmarks** — `benches/hotpath.bcyr` (11 benches, auto-discovered by
  `cyrius bench`) reproduces the intent of `rust-old/benches/benchmarks.rs`. The
  per-sample inner loops are batch-timed (one clock pair per 1000 calls) to remove
  the ~240 ns per-call clock overhead. On x86_64 (single core): glottal
  `next_sample` ≈ **82 ns**, formant filter `process_sample` ≈ **175 ns**, tract
  `process_sample` (Full) ≈ **294 ns** — a full glottal→formant→tract chain ≈
  0.55 µs/sample (~40× real-time at 44.1 kHz). Block/render paths: formant
  `process_block` 1024 ≈ **186 µs**, tract `synthesize_into` 1024 ≈ **375 µs**,
  phoneme synth /a/ ≈ **880 µs**, /s/ ≈ **467 µs**, /ai/ (per-sample formant
  re-solve) ≈ **5.4 ms**, 3-phoneme sequence render ≈ **3.4 ms**.
- **API docs** — doc comments added to all remaining public setters/mutators
  (glottal ×8, tract ×6, sequence ×4, batch renderer ×3, DC blocker, pool reset):
  `cyrdoc --check` now reports **0 undocumented public fns** (was 24).
- **Tooling** — `scripts/bench-history.sh` and `scripts/version-bump.sh` rewritten
  for the Cyrius toolchain (parse `cyrius bench` output → `benches/history.csv`;
  version sourced from `VERSION` via `${file:VERSION}`, no `Cargo.toml`).

### Changed

- **`VERSION` → `3.0.0`** — full-parity release. `math.rs` needs no Cyrius module
  (its `libm`/std shims map directly to `ganita` / f64 builtins).
- Symbol convention: all svara symbols `svara_`/`SVARA_`/`SV_`/`Sv`-prefixed to
  coexist with naad's distlib bundle in one flat namespace.

### Notes (parity)

- f32 → f64 throughout (hisab/naad are f64-only); tolerance parity, not bit-exact.
- `next_f32` ports the Rust **code** ([0.0, 1.0), not the doc's [-1.0, 1.0]).
- svara does not flush denormals (its Rust never did) — none added.
- **Quality gate**: fmt (`cyrfmt`), lint (`cyrlint`, 0 warnings), docs (`cyrdoc`,
  0 undocumented), tests (`cyrius tests tests` → 18/18 suites, 634 assertions), and
  benchmarks (`cyrius bench`) are each green. The aggregate `cyrius audit` command's
  test/bench legs currently report compile errors because that command skips
  dependency resolution before compiling (so the stdlib prelude + hisab/naad/goonj
  bundles are absent) — a toolchain issue reproducible identically in the sibling
  `naad` 2.1.0 release, not a svara defect. Run the individual gate commands instead.

## [2.0.0] - 2026-04-01

### Added

- **Whisper mode** (`GlottalModel::Whisper`): noise-only excitation with no periodic voicing and steep spectral tilt (~12 dB/octave), modeling turbulent airflow through a partially open glottis. `GlottalSource::set_whisper()` convenience method
- **Creaky voice / vocal fry** (`GlottalModel::Creaky`): LF pulse model with irregular period timing — ~40% doubled periods, ~10% tripled periods for subharmonic patterns. 3x shimmer amplification for characteristic amplitude irregularity. `GlottalSource::set_creaky(rd)` with Rd clamped to [0.3, 0.8] (pressed range)
- **Formant bandwidth widening for singing** (`VoiceProfile::bandwidth_widening`): configurable extra bandwidth scaling at high f0 (>300 Hz). Models increased source-tract coupling in singing. Formula: `bw_scale *= 1 + widening * 0.3 * ((f0 - 300) / 500)`. Builder: `with_bandwidth_widening(factor)`, range [0.0, 2.0]
- **Vocal effort continuum** (`VocalEffort` enum): coordinated voice quality control across 5 effort levels (Whisper, Soft, Normal, Loud, Shout). Each level maps to consistent GlottalModel, Rd, breathiness, spectral tilt, f0 range scaling, bandwidth scaling, and jitter/shimmer scaling. `VoiceProfile::with_effort()` builder, `create_glottal_source_with_effort()`, `apply_formant_scale_with_effort()`
- **Anticipatory nasalization** (`Nasalization` struct): vowels preceding nasals (/m/, /n/, /ŋ/) automatically receive gradual nasal coupling ramping up from 65% of the vowel segment. Anti-formant frequency tuned by place of articulation. `synthesize_phoneme_nasalized()` public API
- **Consonant cluster handling**: automatic detection of 2+ adjacent consonants in sequences with 30% duration compression per cluster member. Prevents unnaturally long consonant runs in /str/, /spl/, etc.
- **`SynthesisContext`**: reusable synthesis state (VocalTract + GlottalSource + buffer) for consumers who need to manage allocation. Supports all phoneme classes with nasalization
- **`SynthesisPool`** (`pool.rs`): pre-allocated object pool wrapping `SynthesisContext` with `render`/`render_nasalized`/`render_batch`, pre-warmed buffer via `with_capacity`, diagnostic counters (render_count, peak_samples)
- **`BatchRenderer`** (`render.rs`): non-real-time batch rendering API with `push`/`extend`/`render_all`/`render_with_progress` callback. Concatenates phoneme audio with stress modification and anticipatory nasalization
- **Non-pulmonic consonants**: 13 new phonemes — 5 clicks (ʘ ǀ ǃ ǂ ǁ), 5 ejectives (pʼ tʼ kʼ sʼ tʃʼ), 3 implosives (ɓ ɗ ɠ). New `PhonemeClass` variants: `Click`, `Ejective`, `Implosive`. Click synthesis uses sharp transient bursts shaped by place; ejectives use compressed burst with no aspiration; implosives use creaky-voiced LF pulse with reduced amplitude. Phoneme inventory: 48 → 61
- **Formant trajectory planning** (`trajectory.rs`): `TrajectoryPlanner` computes continuous formant trajectories across 3+ phoneme windows using Catmull-Rom spline interpolation weighted by coarticulation resistance. `PhonemeSequence::render_planned()` synthesizes continuously with per-sample formant updates instead of segment crossfading
- **IPA-complete phoneme inventory** (100 phonemes, up from 48): 7 additional vowels (y, ø, œ, ɯ, ɤ, ɨ, ʉ), 4 plosives (q, ɢ, ʈ, ɖ), 13 fricatives (ɸ, β, ç, ʝ, χ, ʁ, ħ, ʕ, ʂ, ʐ, ɬ, ɮ, ɦ), 3 nasals (ɳ, ɲ, ɴ), 3 trills (ʙ, r, ʀ), 3 approximants/laterals (ɻ, ʎ, ʟ), 2 flaps (ɽ, ɺ), 6 affricates (ts, dz, ʈʂ, ɖʐ, pf, tɬ). New `PhonemeClass::Trill` variant
- **Tone language support** (`Tone` enum): 9 lexical tone patterns — High, Rising, Dipping, Falling, Neutral (Mandarin 5 tones) plus Low, Mid, LowRising, HighFalling for Thai/Vietnamese/African languages. `Tone::to_contour()` produces `ProsodyContour` with f0/duration/amplitude scaling. `PhonemeEvent::with_tone()` constructor
- **Synthesis quality improvements**:
  - `VoiceOnsetTime`: per-phoneme VOT with place-dependent closure/burst/aspiration fractions (Lisker & Abramson 1964). Voiceless stops get long-lag aspiration, voiced stops get short-lag
  - `phoneme_spectral_tilt()`: per-vowel spectral tilt (0-2 dB/oct based on F1 height), available as metadata
  - `height_adjusted_amplitudes()`: F3-F5 attenuate with vowel openness, modeling source-tract coupling (Fant 1960)
  - `GlottalSource::set_speed_quotient()`: speed quotient (0.5-5.0) for asymmetric pulse models
  - `GlottalSource::set_diplophonia()`: alternating strong/weak pulse amplitude for pathological/expressive voice
  - `PhonemeSequence::set_speaking_rate()`: Lindblom undershoot — faster speech reduces coarticulation resistance in trajectory planning
- 2 new benchmarks: `glottal_whisper_1024`, `glottal_creaky_1024`
- 212 total tests (164 unit + 45 integration + 3 doc)

### Performance

- Glottal whisper (1024 samples): 5.0µs (-9% vs Rosenberg, no pulse computation)
- Glottal creaky (1024 samples): 6.9µs (+25% vs Rosenberg, LF pulse + period doubling logic)
- SIMD investigation: manual AVX2+FMA intrinsics benchmarked but `#[target_feature]` call boundary prevents inlining — auto-vectorized loop is faster for runtime-detected paths. Build with `RUSTFLAGS="-C target-cpu=native"` for AVX2 auto-vec: formant filter 4.8µs → 3.8µs (-21%)

## [1.1.1] - 2026-04-01

### Changed

- Removed 7 unused license allowances from `deny.toml` (kept MIT, Apache-2.0, GPL-3.0-only, Unicode-3.0)

### Infrastructure

- Initialized `cargo vet` supply-chain auditing (83 crates exempted)
- Dependency updates: hisab 1.2→1.4, zerocopy 0.8.47→0.8.48, wasm-bindgen 0.2.114→0.2.117, web-sys 0.3.91→0.3.94, js-sys 0.3.91→0.3.94

## [1.1.0] - 2026-03-28

### Added

- **Bridge module** (`bridge.rs`): 18 dependency-free conversion functions for ecosystem integration
  - bhava (emotion/affect): `rd_from_arousal`, `breathiness_from_arousal`, `jitter_from_arousal`, `vibrato_depth_from_valence`, `f0_range_scale_from_arousal`, `intonation_from_emotion`
  - vansh (TTS): `duration_scale_from_speech_rate`, `stress_from_tobi_accent`, `f0_peak_from_prominence`
  - prani (creature): `formant_scale_from_body_size`, `f0_from_body_size`, `jitter_from_age`, `glottal_model_from_effort`
  - goonj (acoustics): `gain_from_distance`, `bandwidth_scale_from_reverb`, `spectral_tilt_from_distance`
  - badal (weather): `lombard_effort_from_noise`, `lombard_f0_shift`, `breathiness_reduction_from_wind`
- **LOD/Quality system** (`lod.rs`): `Quality` enum (Full, Reduced, Minimal) integrated into `VocalTract` for multi-voice CPU scaling
- **Shared RNG module** (`rng.rs`): Deduplicated PCG32 from glottal and phoneme modules
- **Shared DSP utilities** (`dsp.rs`): `validate_sample_rate`, `validate_duration`, `map_naad_error`
- 18 new tests: validation edge cases (NaN, Inf, negative, zero), deterministic replay (single + sequence), streaming API, LOD quality levels — total 45 integration tests (up from 27)
- 13 bridge function tests with range and edge case coverage
- LOD unit tests with serde roundtrips
- no_std testing in Makefile and CI matrix (previously only `--all-features`)
- 4 new examples: `voice_comparison`, `prosody_patterns`, `error_handling`, `streaming`
- 4 Architecture Decision Records: source-filter model, coarticulation model, formant data source, scope boundaries
- Documentation: integration guide, testing guide, dependency watch, threat model
- Send+Sync assertion for `Quality` type

### Changed

- Input validation strengthened: NaN and Infinity now rejected on all public constructors (`GlottalSource::new`, `FormantFilter::new`, `synthesize_phoneme`)
- `VocalTract::process_sample` respects `Quality` setting — skips subglottal, interaction, lip radiation, nasal coupling at lower quality levels
- `GlottalSource`: aspiration noise now uses `naad::noise::NoiseGenerator` (White) when naad-backend enabled; PCG32 fallback otherwise
- `GlottalSource`: vibrato now uses `naad::modulation::Lfo` (Sine) when naad-backend enabled; manual sine fallback otherwise
- `VocalTract`: nasal antiformant now uses `naad::filter::BiquadFilter` (Notch) when naad-backend enabled
- `VocalTract`: subglottal resonance now uses `naad::filter::BiquadFilter` (BandPass) when naad-backend enabled
- `VocalTract`: nasal coupling and gain use `SmoothedParam` one-pole smoother to prevent clicks on real-time parameter changes (5ms time constant)
- `NasalAntiformant` manual implementation gated to `#[cfg(not(feature = "naad-backend"))]`
- CI test matrix now includes `--no-default-features` (ubuntu) alongside `--all-features` (ubuntu + macos)
- SECURITY.md: supported version updated from 0.1.x to 1.x

### Performance

- Vocal tract (Full): 27µs → 23µs (-15%, naad filters more efficient than manual)
- LOD Reduced quality: 15µs (-34% vs Full) — skips subglottal, interaction
- LOD Minimal quality: 14µs (-38% vs Full) — skips nasal coupling, lip radiation too
- 2 new LOD benchmarks added to suite (13 total)

### Infrastructure

- Roadmap updated with v1.1, v1.2, v2.0+ plans
- Forward-looking backlog: whisper/creaky voice, singing, multi-language, SIMD intrinsics

## [1.0.0] - 2026-03-27

### Fixed

- **Spectral tilt**: Replaced constant multiply (no frequency dependence) with a proper one-pole low-pass filter (`y[n] = (1-α)*x[n] + α*y[n-1]`), giving correct frequency-dependent tilt
- **Rosenberg pulse**: Removed non-standard `sqrt(abs(sin))` shaping that deviated from the Rosenberg B model; now pure `3t²-2t³` polynomial. Glottal source benchmark **36% faster** (6.34µs → 4.07µs / 1024 samples)
- **Formant topology naming**: Corrected docs/comments from "cascade" to "parallel bank" (the actual topology: input goes to all filters, outputs are summed)
- **Vowel aliasing**: `VowelOpenA` (/ɑ/) now has distinct formants from `VowelA` (/a/); `VowelBird` (/ɜ/) now has distinct formants from `Schwa` (/ə/)
- **`set_formants()` error propagation**: `VocalTract::set_formants()` and related methods now return `Result` instead of silently ignoring errors
- **License identifier**: `GPL-3.0` → `GPL-3.0-only` in Cargo.toml and deny.toml
- **Doc link**: Escaped `[0,1]` in prosody doc comment to prevent broken intra-doc link

### Added

- **Hillenbrand formant data**: Replaced Peterson & Barney (1952) with Hillenbrand et al. (1995) male averages for all 10 vowels, including per-vowel bandwidths (B1-B5)
- **Per-vowel bandwidths**: `VowelTarget` now stores B1-B5 alongside F1-F5; `with_bandwidths()` constructor; bandwidths interpolated during transitions
- **DC-blocking filter**: `FormantFilter` now includes a one-pole DC blocker (~20 Hz cutoff) to prevent numerical drift
- **Affricates**: `AffricateCh` (/tʃ/) and `AffricateJ` (/dʒ/) with `Affricate` phoneme class and plosive-burst + fricative-release synthesis
- **Glottal stop**: `GlottalStop` (/ʔ/) phoneme
- **Vibrato**: `GlottalSource` now applies sinusoidal f0 modulation using `vibrato_rate` and `vibrato_depth` from `VoiceProfile` (previously defined but never wired up)
- **`VoiceProfile::create_glottal_source()`**: Helper that configures a `GlottalSource` with all voice profile parameters (f0, breathiness, jitter, shimmer, vibrato)
- **`PartialEq`** on `Formant` and `VowelTarget`
- Named constants for magic numbers: `DEFAULT_RNG_SEED`, `DEFAULT_OPEN_QUOTIENT`, `DEFAULT_JITTER`, `DEFAULT_SHIMMER`, `NASAL_ANTIFORMANT_FREQ`, `NASAL_ANTIFORMANT_BW`, `DEFAULT_LIP_RADIATION`, `DEFAULT_BANDWIDTHS`, `DEFAULT_AMPLITUDES`
- 10 serde roundtrip tests: `Vowel`, `FormantFilter`, `VocalTract`, `PhonemeEvent`, `IntonationPattern`, `Stress`, `PhonemeClass`, `SvaraError`, `PhonemeSequence` (deep verify), `VowelTarget`
- **Aspiration noise gating**: Breathiness noise now temporally gated by glottal open phase — full noise during open quotient, reduced during closure
- **Bandwidth scaling by f0**: `apply_formant_scale()` now widens bandwidths proportionally to `sqrt(f0/120)` for female/child voices
- **Tap/flap** `/ɾ/` phoneme (`TapFlap`) — short voiced alveolar contact
- **Look-ahead coarticulation**: Transition to next phoneme begins at configurable onset (default 60% of segment), with sigmoid interpolation curves
- **Coarticulation resistance**: Per-phoneme resistance coefficients (0.0-1.0) based on Recasens DAC model — controls crossfade length at each boundary
- **F2 locus equations**: `f2_locus_equation()` returns (locus, slope) by place of articulation (bilabial, alveolar, velar) per Sussman et al. (1991)
- **Variable crossfade**: Per-boundary crossfade lengths modulated by adjacent phoneme resistance (low-resistance phonemes get longer blending regions)
- **`VocalTract::synthesize_into()`**: Zero-allocation synthesis into pre-allocated buffer
- **SOA formant filter**: Structure-of-arrays `BiquadBankSoa` with fixed `MAX_FORMANTS=8` loop bound enabling compiler auto-vectorization
- **`FormantFilter::process_block()`**: Block-based formant processing for batched audio
- 6 new benchmarks: fricative, diphthong, female vowel, 10-phoneme sequence, pre-allocated tract, block formant filter
- **LF glottal model**: Liljencrants-Fant model with Rd voice quality parameter (0.3=pressed, 1.0=modal, 2.7=breathy). `set_rd()` auto-switches to LF model
- **Source-filter interaction**: Vocal tract impedance feedback modifies excitation signal (configurable 0.0-0.3 strength)
- **Dynamic nasal resonances**: `NasalPlace` enum varies anti-formant by place of articulation (bilabial 750Hz, alveolar 1450Hz, velar 3000Hz)
- **Subglottal resonance**: Tracheal coupling at ~600Hz that interacts with F1 (configurable 0.0-0.2)
- **Gain normalization**: `VocalTract::set_gain()` for output level consistency
- **`no_std` support**: Core DSP works without `std` via `libm` + `alloc`. Enable with `default-features = false`
- **f64 biquad coefficients**: Coefficient computation in f64 prevents quantization errors with narrow bandwidths at high sample rates
- `docs/architecture/overview.md` — module map, data flow, pipeline diagram
- `scripts/bench-history.sh` for tracking benchmark results over time
- `docs/development/roadmap.md` — all v1.0 criteria met

### Changed

- **`FormantFilter` internals**: Refactored from `Vec<BiquadResonator>` (AOS) to `BiquadBankSoa` (SOA) with fixed-size arrays. Formant filter **2x faster** from auto-vectorization of the fixed-bound inner loop
- **`VowelTarget::to_formants()`** now returns `[Formant; 5]` instead of `Vec<Formant>` (zero allocation)
- **`VocalTract::set_formants()`**, `set_formants_from_target()`, `set_vowel()` now return `Result<()>` instead of silently swallowing errors
- **`PhonemeSequence`** now uses variable-length sigmoid crossfades modulated by coarticulation resistance (was fixed-length cosine)
- Phoneme inventory: 48 phonemes (was 44) — added affricates, glottal stop, tap/flap

### Performance

All benchmarks measured at default SSE2. Building with `RUSTFLAGS="-C target-cpu=native"` enables AVX2 for an additional ~20% on formant processing.

- Formant filter (1024 samples): **11.0µs → 5.4µs** (-51%, SOA auto-vectorization)
- Glottal source (1024 samples): **6.34µs → 4.15µs** (-35%)
- Vocal tract (1024 samples): **18.7µs → 12.4µs** (-34%)
- Phoneme render (vowel /a/): **82.7µs → 56.5µs** (-32%)
- Sequence render (5 phonemes): **350µs → 252µs** (-28%)
- Sequence render (10 phonemes): **430µs → 357µs** (-17%)

## [0.1.0] - 2026-03-26

### Added

- Initial scaffold of the svara crate
- `GlottalSource`: Rosenberg glottal pulse model with f0, open quotient, spectral tilt, jitter, shimmer, breathiness
- `FormantFilter`: Cascade biquad resonator bank with parallel summing
- `VowelTarget`: Peterson & Barney (1952) formant frequencies for 10 vowel categories with F1-F5
- `VowelTarget::interpolate`: Linear interpolation between vowel targets for smooth transitions
- `VocalTract`: Formant filtering + nasal coupling (anti-formant at 250Hz) + lip radiation (first-order HPF)
- `Phoneme` enum: 44 phonemes (15 vowels, 5 diphthongs, 6 plosives, 9 fricatives, 3 nasals, 4 approximants/laterals, silence)
- `PhonemeClass` enum: Plosive, Fricative, Nasal, Approximant, Lateral, Vowel, Diphthong, Silence
- `synthesize_phoneme`: Class-specific synthesis (vowels via glottal+tract, fricatives via filtered noise, plosives via burst+aspiration, nasals via nasal coupling, diphthongs via formant interpolation)
- `ProsodyContour`: Time-value f0 contour with linear interpolation
- `IntonationPattern`: Declarative (falling), Interrogative (rising), Continuation (rise-fall), Exclamatory (high-fall)
- `Stress` enum with f0/duration/amplitude modifications
- `VoiceProfile`: Male (120Hz), female (220Hz, 1.17x formant scale), child (300Hz, 1.3x) presets with builder pattern
- `PhonemeSequence`: Ordered phoneme events with coarticulation crossfading at boundaries (configurable 50ms window)
- `SvaraError`: InvalidFormant, InvalidPhoneme, InvalidPitch, InvalidDuration, ArticulationFailed, ComputationError
- Integration tests: spectral energy, glottal period, click detection, serde roundtrips, interpolation endpoints
- Criterion benchmarks: glottal, formant filter, vocal tract, phoneme render, sequence render
- Feature flags: `naad-backend` (default), `logging`
- CI/CD: GitHub Actions workflows for test, lint, coverage, release
