# Satoshi's Razor

Infrastructure for massively parallel search over proofs: a canonical public registry
of open formal problems with machine-checkable admission, so that anyone willing to
dedicate AI compute - or human effort - can attack the mathematical frontier in
parallel, without duplicating work and with verifiable attribution.

Folding@home for mathematics.

## Working demo

This repo contains a working implementation of the registry, the funnel, and the
Anvil - real Lean verification, real fuel-metered benchmarks, no chain.

```sh
./install.sh                           # checks toolchain, builds, replays the demo, serves the site
# or step by step:
./seed.sh                              # the live registry: real corpora + real open problems, no fiction
./demo.sh                              # OR the scripted walkthrough exercising every mechanism
target/release/razor serve             # then browse the registry at http://localhost:8420 (live: data.json is re-derived from the log per request)
```

Two datasets. `./seed.sh` builds the **live** registry: recognized prior corpora with
sourced statistics (Mathlib, DeepMind's formal-conjectures, Physlib) and real open
problems from the Lean community's actual frontier, entered as proposals - the boards
are otherwise empty because nothing fictional is recorded. `./demo.sh` builds the
**demo** dataset: fictional participants walking every mechanism end to end (the site
shows a banner when it is loaded). All amounts anywhere are denominated in credits, a
hypothetical accounting unit - the registry maintains a ledger and moves no money.

The demo registers holes with pinned Lean statements, verifies submissions with the
actual Lean kernel (rejecting `sorry` and wrong statements), walks a weak statement
through the full funnel (a funder trusts an unconverged statement and puts a bounty
on its exact wording, a two-line proof is found, admitted, and paid - the fidelity
risk lands on the funder, priced; two independent corrected formalizations converge
into a dominant clump, decomposition glued, the clump's version funded and solved),
then runs two
Anvil challenges where SWAR popcount and closed-form summation - each admitted by a
machine-checked refinement proof, one settled by SAT, one by induction - take the
crowns on deterministic wasm-fuel and native leaderboards.

Layout: `lean/` (specs, proofs, funnel case studies) · `registry/` (the `razor` CLI:
event log, lifecycle, verifier) · `anvil/` (challenge ABI, four implementations, fuel
+ native harness) · `site/` (the frontier explorer) · [FUNNEL.md](FUNNEL.md) and
[ANVIL.md](ANVIL.md) (design).

## The core idea

Mathematical discovery is becoming a search problem. As AI systems approach and exceed
PhD-level proving ability, the binding constraints are no longer talent and insight
alone but: which problems are worth attacking, who is already attacking them, and how
to check a claimed solution without trusting the claimant.

Satoshi's Razor answers all three with one object: a **registry of holes**. A hole is
a Lean theorem statement with a `sorry` body - a precisely specified gap in
formalized mathematics. The registry makes every hole:

- **Targetable**: stated in a standard, machine-readable format that any human or AI
  agent can immediately attempt.
- **Checkable**: a submission is a Lean proof term; admission is a kernel check, not a
  referee's opinion. Anyone can re-run it. Peer review is replaced by verification.
- **Attributed**: solutions are timestamped on an append-only public record, so
  priority and credit are cryptographically established rather than socially disputed.
- **Non-duplicated**: agents can see what is open, what is claimed, what is solved,
  and what partial progress (decomposed subgoals, intermediate lemmas) exists to
  build on - so a thousand agents don't independently burn compute on the same lemma.

The blockchain's role is deliberately narrow: a neutral, append-only, verifiable record
of who solved what, first, exactly as stated. That is what chains are genuinely good
at. Everything bulky (proof artifacts, package contents) lives in content-addressed
storage with hashes on-chain.

## Why now

- **Formalization is reaching critical mass.** The Lean community's progress -
  [Mathlib](https://leanprover-community.github.io/), the
  [Xena Project](https://www.ma.imperial.ac.uk/~buzzard/xena/), efforts like
  [con-nf](https://leanprover-community.github.io/con-nf/) and the
  [PFR blueprint](https://teorth.github.io/pfr/blueprint/dep_graph_document.html) -
  shows that serious mathematics can be formalized at scale, and that large
  formalization projects naturally decompose into exactly the kind of dependency
  graphs of small holes this registry hosts.
- **AI provers have arrived.** Automated and LLM-driven proof search now solves
  nontrivial holes, and its throughput scales with compute. What that compute lacks
  is a shared frontier: a canonical list of what is open, in a format it can consume
  directly.
- **Verification is trustless.** A Lean proof checks or it doesn't. This is the rare
  domain where contributions from anonymous strangers and unaligned AI systems can be
  accepted at zero trust - the perfect substrate for permissionless parallelism.
- **Prior art was early.** [MathCoin](https://eprint.iacr.org/2018/271.pdf),
  [ProofMarket](https://web.archive.org/web/20140110015900/https://proofmarket.org/problem/recent),
  and [Pi²](https://harmonic.fun/news.html) explored adjacent ideas before the
  tooling existed. The tooling exists now.

## What the protocol provides

### The hole registry
- On-chain registration of problem statements: conjectures, lemmas, formalization
  tasks, and FV challenges, each pinned to exact toolchain and dependency versions.
- Splits: partial progress (a proof of a hole with `sorry` in the gaps) is
  registered as child holes plus a machine-checked glue proof that the children
  jointly imply the parent, forming a public dependency graph. Several splits
  of one hole coexist; solving a child is a first-class, attributed
  contribution.
- Claim signaling: agents can (optionally, non-exclusively) announce they are working
  on a hole, so search effort spreads across the frontier instead of piling up.
- A pipeline from informal conjecture to convergent, machine-checkable statement -
  proposal, formalization, adversarial challenge windows, decomposition, and
  prioritization signals - specified in [FUNNEL.md](FUNNEL.md).

### Verification and attribution
- Submissions are checked by the Lean kernel against the pinned statement - no extra
  axioms, no `sorry`. Admission can be verified on-chain via ZK proofs of the checker
  run (the [Yatima](https://github.com/argumentcomputer/yatima) route) or
  optimistically with a fraud-proof window.
- First valid submission takes priority, permanently and publicly. Commit-reveal
  submission prevents front-running of pending solutions.
- The result is a leaderboard culture: for both academics and AI labs, verifiable
  credit on a public frontier is the reward - the same motivator that drives every
  benchmark leaderboard today.

### Trustless package management
- Solved holes become versioned, content-addressed Lean packages others can build on:
  an immutable, generation-preserving record of proven mathematics with automated
  dependency tracking.
- A searchable knowledge base (Hoogle-like interface over statements) so agents and
  humans can find what is already proven before attempting it.

### Value: curation first, bounties as an optional edge
The protocol is free and permissionless; money is a module, not the identity.
Importance is assigned primarily by **curation**: signed, costless, timestamped
picks, weighted by the curator's verified work, so taste is scoreable the way
proofs are. On top of that, anyone who concretely values a proof of one *exact
pinned statement* can attach a **bounty** to it - never to a proposal. The first
admitted proof of the literal statement is paid, degenerate proofs included, with
no adjudication: the funder carries the fidelity risk, informed by the statement's
clump weight, gloss, and certificates. This is how commercial formal verification
work (crypto implementations, safety-critical systems, protocol verification) can
fund the same frontier. Nothing about the registry, verification, or attribution
requires credits at all.

This layering matters: AI inference costs real money, so sustained large-scale search
benefits from funding - but the formalization community should never need to touch a
token to state, solve, or build on a hole.

## Who uses it

- **AI labs and independent compute donors**: point proof-search agents at the open
  frontier; the registry is the shared work queue, the checker is the filter, the
  record is the benchmark.
- **Mathematicians**: decompose formalization projects into public holes and let the
  world - human and machine - fill them in parallel, with attribution preserved.
- **Commercial FV teams**: break large verification tasks into modular, priced holes
  and accept solutions from anyone, with correctness guaranteed by the checker rather
  than by vendor trust.
- **Educators and students**: a progression path of real, attributed contributions,
  from small lemmas upward.

## Sister project

[Satoshi's Anvil](ANVIL.md) applies the same architecture to programs instead of
proofs: holes are formal specifications, submissions are Rust implementations proven
to refine them, and admitted implementations compete on per-architecture performance
leaderboards. Razor searches for truth; Anvil searches for efficient certified
computation. Together they form a pipeline: formalize a problem (Razor bounty), prove
the theory (Razor hole), then implement it fast and verified (Anvil challenge).

## Architecture

- **Contracts**: hole registry, verification/attribution record, optional reward
  escrow, dependency graph.
- **Verification pipeline**: pinned Lean toolchain containers; ZK checker proofs
  (Yatima) or optimistic re-execution.
- **Storage**: content-addressed proof and package artifacts (IPFS/Arweave), hashes
  on-chain.
- **Frontend**: frontier explorer (open holes, dependency graphs, claim signals),
  proof browser, knowledge-base search, submission tooling.
- **Agent SDK**: a standard interface for proof-search agents to poll the frontier,
  fetch hole contexts, and submit candidates - designed so pointing compute at the
  registry is a one-day integration.

## Roadmap

### Phase 1: The registry (months 1-6)
- Hole registry and verification pipeline on testnet; optimistic admission.
  (Already in the reference implementation: Ed25519-signed events with
  `verify-log` auditing, and sandboxed no-network verification with a time
  limit.)
- Agent SDK and frontier explorer.
- Seed the frontier: import open `sorry`s from willing public formalization projects.
- Success metric: independent agents solving registry holes that feed back upstream.

### Phase 2: Attribution and scale (months 7-12)
- Zk routes (per-hole theorem-bridged circuits now; the universal checker-in-a-zkVM route once benchmarked); commit-reveal submissions.
- Decomposition and dependency-graph tooling; knowledge-base search.
- Optional reward-pool module launches, kept strictly separable.
- Lean Zulip and community integrations.

### Phase 3: The pipeline (months 13-24)
- Additional proof assistants.
- Satoshi's Anvil shared infrastructure (spec bounties feeding implementation
  challenges).
- Sustained-funding mechanisms for compute donors (grants, public-goods funding,
  commercial pools).

## Contributing

Early stage, actively seeking contributors in formal verification, smart contract
development, agent tooling, frontend, and documentation.

## Contact

Reach out on github or dm @mempoolsurfer on telegram.

## License

MIT - see [LICENSE](LICENSE).
