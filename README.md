# Satoshi's Razor

A public registry of open formal problems. A hole is a Lean theorem statement
with a `sorry` body: a precisely specified gap in formalized mathematics.
Anyone - human or AI - can attempt one, and admission is a kernel check, not a
referee's opinion. Every event lands on an append-only log, so priority and
credit are verifiable rather than socially disputed.

Holes are stated in Mathlib's vocabulary by default, and where Mathlib
already names the Prop the hole pins Mathlib's own name - the FLT hole pins
`FermatLastTheorem` itself, so the proof the Imperial FLT project lands in
Mathlib closes it verbatim. A proof admitted here is a proof the rest of
formal mathematics can build on, not one stranded in a private dialect.

## Quick start

```sh
./install.sh       # checks the toolchain, builds, links razor / anvil-harness / zk-prover onto PATH
./seed.sh          # the live registry: real corpora and real open problems, no fiction
./demo.sh          # OR the scripted walkthrough exercising every mechanism (fictional participants)
razor serve        # browse the registry at http://localhost:8420
./mathlib-env.sh   # once, to verify Mathlib-environment holes locally (several GB of prebuilt cache)
```

`razor help` lists every command. The registry's entire state is one file,
`registry/data/events.jsonl`; everything else - the site, the leaderboards,
every profile - is derived from it. CI elaborates every pinned Mathlib
statement on every push, so you do not need the local Mathlib cache to trust
the statements.

## What is on the frontier

- Flagship holes in Mathlib vocabulary: Fermat's Last Theorem
  (`FermatLastTheorem`, Mathlib's own Prop), Erdos-Straus, Erdos-Turan on
  additive bases.
- A sourced backlog ingested from the [1000+ theorems
  list](https://1000-plus.github.io/): as of 2026-07-03, 975 catalogued
  theorems with no Lean formalization, each a proposal waiting for someone to
  pin its statement. Refresh with `uv run ingest/fetch_thousand_plus.py`.
- Recognized corpora (Mathlib, formal-conjectures, Physlib): work that is
  already done, never re-proved - a hole that duplicates it closes by
  citation.

## How it works

- **From words to a hole.** A problem enters as a plain-language proposal;
  candidate Lean statements are filed against it; statements proven equivalent
  by machine-checked proof clump together. Two independent authors converging
  is the strongest evidence a formalization is faithful. The full pipeline is
  specified in [FUNNEL.md](FUNNEL.md).
- **Solving.** `razor submit --file proof.lean` takes a single Lean file; the
  verifier checks the named declaration against the hole's exact pinned
  statement - no `sorry`, no extra axioms - inside a no-network sandbox.
- **Partial progress is attributed.** A hole can be split into child holes
  plus a glue hole whose statement the CLI composes mechanically - `(child 1)
  → ... → (child n) → parent` - so an admitted glue proof is a kernel-checked
  fact that the children suffice, and each child solve is credited on its own.
- **Statements survive churn.** When a library refactor respells a pinned
  statement, `razor repin` migrates the hole - but only if a proof that the
  two wordings are equivalent kernel-checks. Old wording, new wording, and the
  equivalence stay on the log; prior admissions remain valid.
- **Attribution you can take with you.** Registered handles sign events with
  Ed25519 keys (`razor verify-log` audits the whole log), can bridge to a
  GitHub identity, and `razor cite` emits a citation pinning an admitted proof
  to an event number and a log hash anyone can recheck. Commit-reveal and
  zero-knowledge routes keep a pending proof private without giving up
  priority.
- **Value.** Importance is assigned by curation: signed, costless picks,
  weighted by the curator's admitted work. Anyone who values a proof of one
  exact statement can attach a bounty; the first admitted proof is paid,
  degenerate proofs included - the funder carries the fidelity risk. Nothing
  about stating, solving, or building on a hole requires credits.
- **The Anvil.** The same machinery for programs: holes are formal
  specifications, submissions are implementations proven to refine them, and
  admitted implementations compete on fuel-metered and native leaderboards.
  See [ANVIL.md](ANVIL.md).

## Layout

| Path | What it is |
|---|---|
| `registry/` | the `razor` CLI: event log, verifier, signatures, site server |
| `lean-mathlib/` | the Mathlib environment - the default vocabulary for new holes |
| `lean/` | the dependency-free core environment (glue proofs, demos) |
| `ingest/` | catalogue ingestion (1000+ theorems snapshot + fetcher) |
| `zk/` | Groth16 prover/verifier for zero-knowledge routes |
| `anvil/` | challenge specs, implementations, fuel + native harness |
| `site/` | the frontier explorer, computed from the log |

## Contributing

Early stage, actively seeking contributors in formal verification, agent
tooling, frontend, and documentation. Reach out on github or dm
@mempoolsurfer on telegram.

MIT - see [LICENSE](LICENSE).
