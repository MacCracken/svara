# Architecture notes

Non-obvious constraints, quirks, and invariants that a reader cannot derive from the code alone. Numbered chronologically — never renumber.

Not decisions (those live in [`../adr/`](../adr/)) and not guides (those live in [`../guides/`](../guides/)). An item here describes *how the world is*, not *what we chose* or *how to do something*.

## Items

- [`overview.md`](overview.md) — the synthesis pipeline, the module map, and the two constraints that shape the code (the bump allocator never frees; Cyrius emits scalar code).

No numbered entries yet. Add one (`001-kebab-case-title.md`) the first time the code has a non-obvious invariant a reader can't derive. Do not write entries for decisions — those are ADRs.

> Four ADRs used to live in this directory (`adr-001`…`adr-004`), contradicting the
> line above. They moved to [`../adr/`](../adr/) as 0003–0006 on 2026-08-26.
