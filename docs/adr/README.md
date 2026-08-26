# Architecture Decision Records

Decisions about svara — what we chose, the context, and the consequences we accept. Use these when a future reader would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNNN-kebab-case-title.md`, zero-padded to four digits. Never renumber.
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to `Superseded by NNNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adr/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-signed-index-and-float-conversion-hazards.md) | Signed-index and float-conversion hazards inherited from the Rust port | Accepted |
| [0002](0002-serialization-boundary.md) | Where serialization stops | Accepted |
| [0003](0003-source-filter-model.md) | Source-filter model choice | Accepted |
| [0004](0004-coarticulation-model.md) | Coarticulation model | Accepted |
| [0005](0005-formant-data-source.md) | Formant data source | Accepted |
| [0006](0006-scope-boundaries.md) | Scope boundaries | Accepted |

0003–0006 predate the Cyrius port (Rust v1.0.0) and were re-homed here from
`docs/architecture/` on 2026-08-26; they are domain decisions and survive the port.
