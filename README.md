# Satoshi's Razor

A public registry of open formal problems. A hole is a Lean theorem statement
with a `sorry` body: a precisely specified gap in formalized mathematics.
Anyone - human or AI - can attempt one, and admission is a kernel check, not a
referee's opinion. Every event lands on an append-only log, so priority and
credit are verifiable rather than socially disputed.

## Quick start

```sh
./install.sh   # checks the toolchain, builds, links razor / anvil-harness / zk-prover onto PATH
./seed.sh      # the live registry: real corpora and real open problems, no fiction
./demo.sh      # OR the scripted walkthrough exercising every mechanism (fictional participants)
razor serve    # browse the registry at http://localhost:8420
```

`razor help` lists every command. The registry's entire state is one file,
`registry/data/events.jsonl`; everything else - the site, the leaderboards,
every profile - is derived from it.

## How it works

- **From words to a hole.** A problem enters as a plain-language proposal;
  candidate Lean statements are filed against it; statements proven equivalent
  by machine-checked proof clump together. Two independent authors converging
  is the strongest evidence a formalization is faithful. The full pipeline is
  specified in [FUNNEL.md](FUNNEL.md).
- **Solving.** A submission names a Lean declaration; the verifier checks it
  against the hole's exact pinned statement - no `sorry`, no extra axioms -
  inside a no-network sandbox. A hole can also be split into child holes plus
  a machine-checked glue proof, so partial progress is public and attributed.
- **Attribution.** Registered handles sign their events with Ed25519 keys and
  `razor verify-log` audits the whole log. Commit-reveal keeps a pending proof
  private without giving up priority; zero-knowledge routes let a solver prove
  they hold a witness without revealing it at all.
- **Value.** Importance is assigned by curation: signed, costless picks,
  weighted by the curator's admitted work. Anyone who values a proof of one
  exact statement can attach a bounty to it; the first admitted proof is paid,
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
| `lean/`, `lean-mathlib/` | the Lean packages the registry pins statements in |
| `zk/` | Groth16 prover/verifier for zero-knowledge routes |
| `anvil/` | challenge specs, implementations, fuel + native harness |
| `site/` | the frontier explorer, computed from the log |

## Contributing

Early stage, actively seeking contributors in formal verification, agent
tooling, frontend, and documentation. Reach out on github or dm
@mempoolsurfer on telegram.

MIT - see [LICENSE](LICENSE).
