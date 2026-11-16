# Two threads rendering at once, each on its own context, sometimes render the wrong samples — OPEN

**Status:** 🟡 **OPEN**. Nothing in svara is changed.
**Placement:** roadmap 3.7.x, "Concurrent renders can give wrong samples". Since 3.6.4 the integration guide says to serialize every call (the issue's option 2) until the scratch moves into the objects (option 1).
**Re-verified:** against 3.6.4 on 2026-11-16: `wrong renders of 4000 concurrent: 7`, exit 7.
**Where the fix would live:** the module-level scratch that one render writes and then reads back:
- `_sv_tract_target_scratch` (`src/tract.cyr:263`), written by `svara_tract_set_formants_from_target` (`:273-275`)
  and read straight back by `svara_tract_set_formants`;
- `_sv_glottal_rd_out` (`src/glottal.cyr:69`), written by `svara_lf_params_from_rd` in `svara_glottal_apply_rd`
  (`:75`) and read back into the source;
- by the same pattern, `_sv_prosody_xs` / `_sv_prosody_ys` / `_sv_prosody_cap` (`src/prosody.cyr`) and the
  `_sv_spec_*` buffers (`src/spectral.cyr:98-103`). The repro exercises only the first two.

**Discovered:** 2026-11-11, building hoosh 2.12.2's `POST /v1/audio/speech` (hoosh's ADR 016).
**Reporter:** hoosh 2.12.2
**Severity:** Medium.
- **Why it matters.** The wrong render returns success. It is a silent wrong answer, and a rare one: 2 to 4 of 4,000
  renders in the repro.
- **Who can reach it.** Any consumer that renders on more than one thread. hoosh's gateway runs seven workers. It
  holds one lock around every call into svara, so it is not affected, but nothing in svara's docs says a lock is
  needed.

**Affects:** svara 3.6.3 (tag `3.6.3`, `7783d18`), built with cyrius 6.7.18.

## Summary

Each render in the repro is 50 ms of a vowel on a fresh 24 kHz synthesis context (`svara_synthctx_new`, then
`svara_synthctx_synthesize`), checksummed. On one thread, the same vowel always gives the same checksum. Four threads
then render 1,000 each, two /a/ and two /i/, and count the renders whose checksum differs from the one rendered
alone:

```
deterministic alone: 1
wrong renders of 4000 concurrent: 2
exit=2
```

hoosh's first run of the same test, built in its own tree, counted 3 or 4 of 4,000.

Nothing is shared between the two threads' contexts, voices or buffers. What is shared is the scratch above: one
thread writes a target's formants into `_sv_tract_target_scratch`, another overwrites them, and the first sets its
tract from the second's formants. The lazy `== 0` initialization of each scratch races too, which costs a leaked
buffer, not a wrong answer.

## Reproduction

[`repros/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.cyr`](repros/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.cyr).
It needs the stdlib's thread module, which svara's `lib/` does not vendor:

```sh
for f in thread thread_win thread_macos thread_agnos mmap sync sync_macos sync_windows; do
  cp ~/.cyrius/versions/6.7.18/lib/$f.cyr lib/; done
cyrius build docs/development/issues/repros/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.cyr /tmp/race
/tmp/race; echo "exit=$?"
```

The exit code is the number of wrong renders: 0 when concurrent renders are safe.

## What would fix it

Either of:
1. **Scratch per object.** Give each tract its own target scratch, and each glottal source its own LF triple,
   allocated with the object. That costs a few dozen bytes per object and makes every render that shares nothing
   safe to run at once. A context built per request (hoosh's pattern) would carry them.
2. **Document the constraint.** If module scratch stays, say in the integration guide that svara renders one call at
   a time per process, so a multi-threaded consumer must serialize every call.

## References

- hoosh `docs/decisions/016-in-process-speech-through-svara.md` (decision 5, the lock) and `src/lib/svara_local.cyr`.
- `2026-11-11-a-server-render-keeps-what-each-phoneme-allocates.md` (filed with this).
