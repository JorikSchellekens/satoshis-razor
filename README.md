# Satoshi's Razor

A public registry of open formal problems. Browse it at
[razor.mempoolsurfer.com](https://razor.mempoolsurfer.com).

A **sorry** is an open problem stated exactly: a Lean theorem statement whose
proof is missing. Anyone - a person or a machine - can submit a proof, and
the Lean kernel decides whether it is accepted; no referee does. Every
action is recorded on an append-only log, so who stated, proved, funded, or
curated what, and when, is a fact anyone can check.

Sorries are stated with Mathlib's definitions by default. Where Mathlib
already defines the statement itself, the sorry uses Mathlib's own name: the
Fermat's Last Theorem sorry is `FermatLastTheorem`, the exact statement the
Imperial College FLT project is proving, so the proof that lands in Mathlib
closes the sorry with no translation step in between. Because sorries are
stated in the language the rest of formal mathematics already uses, an
accepted proof can be contributed onward to Mathlib.

The registry is built to answer two questions:

- **Does this Lean statement say what the informal theorem says?** No
  machine can check that, so the registry records the evidence instead: how
  many people independently wrote a Lean statement for the same problem,
  whether those statements were proven equivalent by the kernel, and which
  sanity checks passed. When two people translate the same sentence into
  Lean without seeing each other's work and the results are proven
  equivalent, a mistranslation is unlikely - and here that evidence is
  recorded and queryable.
- **Did that machine really solve that open problem?** A claimed solve
  reduces to one command. `razor recheck` re-runs the kernel check against
  the statement as it was pinned before the attempt, checks the signature
  on the claim, and compares the result with the verdict on the log.

## Quick start

The hosted registry is at
[razor.mempoolsurfer.com](https://razor.mempoolsurfer.com). To run
everything locally:

```sh
./install.sh       # checks the toolchain, builds, puts razor / anvil-harness / zk-prover on PATH
razor serve        # browse the live registry at http://localhost:8420
./demo.sh          # optional: a scripted walkthrough of every mechanism, with fictional participants
./mathlib-env.sh   # once, if you want to verify Mathlib-environment sorries locally (several GB)
```

`razor help` lists every command. The registry's entire state is one file,
`registry/data/events.jsonl`, committed to this repository - a fresh clone
already contains the live registry, so there is nothing to load. The site,
the leaderboards, and every profile are computed from that file, anyone
with a checkout can verify a citation without trusting a server, and CI
compiles every pinned Mathlib statement on every push. (demo.sh overwrites
the log locally; restore the real one with
`git checkout registry/data/events.jsonl`. `./seed.sh` regenerates the
live dataset from scratch with fresh timestamps - a maintenance tool, not
a setup step.)

## Participate in the hosted registry

The CLI is the interface: after `./install.sh`, participation commands
publish directly to [razor.mempoolsurfer.com](https://razor.mempoolsurfer.com).
`razor account new` registers your handle there; `razor formalize`,
`razor seal-statement`, `razor submit --file proof.lean` and the rest do
what they say, signed by your local key, with proof submissions
kernel-checked on the spot in a throwaway container. `razor status` shows
the live registry in your terminal.

Your signing key lives in `~/.config/razor/keys/` - outside the clone, so
no script can touch it. Back it up: it signs everything you do under your
handle and cannot be regenerated.

The registry is a sequencer, not an authority. It cannot forge your events
(your key signs them and never leaves your machine), and its verdicts are
re-checkable by anyone: the log is mirrored to this repository on every
append, and `razor recheck` replays any kernel check on your own machine.
To work entirely on your machine instead, run `razor remote off`, or add
`--local` to any single command.

## What is in the registry

- Open problems stated with Mathlib's definitions: Fermat's Last Theorem
  (`FermatLastTheorem`, Mathlib's own statement), the Erdos-Straus
  conjecture, and the Erdos-Turan conjecture on additive bases.
- 975 catalogued theorems with no Lean formalization, imported from the
  [1000+ theorems list](https://1000-plus.github.io/) (snapshot dated
  2026-07-03). Each is a proposal waiting for someone to write its Lean
  statement. Refresh the snapshot with `uv run ingest/fetch_thousand_plus.py`.
- Recognized prior work (Mathlib, formal-conjectures, Physlib). The registry
  does not restate or re-prove it, and a sorry that turns out to duplicate it
  is closed by citing it.

## How it works

- **From words to a sorry.** A problem starts as a proposal in plain
  language. Anyone can file a candidate Lean statement for it, together
  with a gloss: the author's own plain-language reading of their Lean.
  Statements proven equivalent by the kernel are grouped into clumps.
  During a challenge window, statements can be filed sealed - a hash first,
  the statement itself later - so statements sealed before one another's
  reveals were provably written without seeing each other. The equivalence
  of two statements can itself be registered as a sorry, called a bridge, so
  proving it is credited and checked like any other proof. The full
  pipeline is described in [FUNNEL.md](FUNNEL.md).
- **Solving.** `razor submit --file proof.lean` takes one Lean file. The
  verifier checks the named declaration against the sorry's exact pinned
  statement - no `sorry`, no extra axioms - in a sandbox with no network
  access. `razor recheck` re-runs that check later, read-only, and compares
  the result with the recorded verdict.
- **Contributing proofs to Mathlib.** `razor upstream` turns an accepted
  proof into a draft Mathlib contribution, with a header recording the
  submission, the verdict event, and the log hash. Once the pull request
  lands, the sorry shows as upstreamed. The registry measures itself by
  upstreamed proofs, not accepted ones.
- **Exports for automated provers.** `razor export-benchmark` writes every
  open sorry as a JSONL proving target in the format prover benchmarks
  already use: an import header, a formal statement ending in `sorry`, and
  the informal text. Claimed solves come back through submit, verify, and
  recheck.
- **Partial progress.** A sorry can be split into child sorries plus a glue
  sorry, whose statement the CLI composes mechanically:
  `(child 1) → ... → (child n) → parent`. An accepted glue proof is a
  kernel-checked fact that solving the children solves the parent, and each
  child solve is credited on its own.
- **Library changes.** When a Mathlib refactor renames or respells a pinned
  statement, `razor repin` migrates the sorry to the new wording - but only
  if a proof that the two wordings are equivalent passes the kernel. The
  old wording, the new wording, and the equivalence proof stay on the log,
  so earlier accepted proofs remain valid.
- **Identity and citations.** A registered handle signs its events with an
  Ed25519 key, and `razor verify-log` checks every signature on the log.
  `razor cite` prints a citation that pins a proof to an event number and a
  log hash. A solver can also commit a hash of a proof and reveal the proof
  later, or submit a zero-knowledge proof, to establish priority without
  showing the proof itself.
- **Money and importance.** Importance is assigned by curation: public,
  signed picks of problems worth working on, weighted by the curator's own
  accepted work. Anyone can attach a bounty to one exact statement, and the
  first accepted proof of that literal statement is paid, trivial proofs
  included - so the risk that the statement was badly worded stays with the
  funder, who could see the recorded evidence before spending. Nothing
  about stating, solving, or building on a sorry requires credits.
- **The Anvil.** The same machinery applied to programs: the specification
  is formal, submissions are implementations with a proof that they refine
  the specification, and accepted implementations compete on measured
  speed. See [ANVIL.md](ANVIL.md).

## Layout

| Path | What it is |
|---|---|
| `registry/` | the `razor` CLI: event log, verifier, signatures, site server |
| `lean-mathlib/` | the Mathlib environment - the default home for new sorries |
| `lean/` | the dependency-free core environment (glue proofs, demos) |
| `ingest/` | catalogue ingestion (1000+ theorems snapshot + fetcher) |
| `zk/` | Groth16 prover/verifier for zero-knowledge routes |
| `anvil/` | challenge specs, implementations, fuel + native harness |
| `site/` | the site at razor.mempoolsurfer.com, computed from the log |

## Contributing

Early stage, actively seeking contributors in formal verification, agent
tooling, frontend, and documentation. Reach out on GitHub or message
@mempoolsurfer on Telegram.

MIT - see [LICENSE](LICENSE).
