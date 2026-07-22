# Satoshi's Anvil

**Running today** at [razor.mempoolsurfer.com/anvil.html](https://razor.mempoolsurfer.com/anvil.html):
six challenges (popcount, sum, sort8, count-leading-zeros, bit-reversal, a small EVM
interpreter), every contender admitted by a kernel-checked refinement proof, scored on
the deterministic wasm-fuel referee and on real registered rigs (an Apple M3 Pro, a
Docker Linux container, and the x86-64 server that hosts the site, benched over ssh).
One lane runs its whole input stream on the GPU - the same proven 19-comparator
sorting network as a compute shader - and takes the native crown once the batch is
large enough to amortize the dispatch. Scores are log events signed by the rig owner;
`razor challenge`, `razor anvil-submit`, `razor rig`, and `razor bench --rig` all
publish to the live registry.

Infrastructure for massively parallel search over programs: given a formal
specification in Lean, anyone willing to dedicate AI compute - or human skill -
submits Rust implementations proven to refine that spec, and admitted
implementations compete on execution performance per target architecture.
Correctness is a machine-checked admission requirement; speed is the score.

Sister project to [Satoshi's Razor](README.md). Razor coordinates the search for
proofs; the Anvil coordinates the search for fast implementations that are proven
correct.

## One-paragraph pitch

Today, if you want a fast AND correct implementation of a cryptographic primitive, a
codec, a serialization format, or a numerical kernel, you choose between audited-but-slow
reference code and fast-but-trust-me assembly. Satoshi's Anvil publishes Lean
specifications as an open frontier of implementation challenges, and anyone - human or
AI - submits a Rust implementation together with a Lean proof that the implementation
refines the spec. Submissions that pass verification enter a per-architecture
performance leaderboard (x86-64, aarch64, RISC-V, wasm32, GPU). The leaderboard itself
is the reward: verifiable, attributed championship per architecture - with optional
reward pools attachable by anyone who values a challenge enough to fund it. The output
is a growing public library of proven-correct, competitively-optimized implementations
that superoptimizing AI search can safely contribute to, because any candidate is
either proven correct or rejected.

## Why this is now possible

The load-bearing technical fact: the Rust-to-Lean verification pipeline exists and is
maturing fast.

- **Aeneas + Charon** (Inria/AWS): translates safe Rust (via the LLBC intermediate
  representation) into pure functional Lean code, against which refinement theorems can
  be stated and proven in ordinary Lean 4. Already used to verify real cryptographic
  code.
- **Verus / Kani / Creusot**: alternative Rust verification tracks; the platform can
  admit multiple verification backends per challenge, as Razor admits multiple proof
  assistants.
- **Lean 4 + Mathlib**: rich enough to state real-world specs (number theory for crypto,
  bit-level semantics, floating-point models).
- **AI code generation**: superoptimization by LLM-driven search is exactly the kind of
  work a reward pool can direct. A formal spec makes the search safe: any candidate the
  AI produces is either proven correct or rejected. This removes the single biggest
  blocker to deploying AI-generated systems code in production.

## Core objects

### Challenge

Posted on-chain by a sponsor. Contains:

- **Spec package**: a Lean package pinning:
  - the functional specification `spec : Input -> Output` (or a relational spec
    `R : Input -> Output -> Prop`),
  - the input domain and validity predicate,
  - the exact refinement obligation the submitter must prove, as a named theorem
    signature with a `sorry` body (the "hole"). Example:
    `theorem refines (x : Input) (h : Valid x) : impl x = spec x`.
- **Interface contract**: the exact Rust `extern`/crate API the implementation must
  export (types, ownership discipline, allowed dependencies, `no_std` or not,
  permitted unsafe policy - default: no `unsafe`, since Aeneas covers safe Rust).
- **Benchmark harness**: pinned input-generation seed procedure, workload sizes,
  warm-up policy, and the scoring formula. The harness itself is part of the spec and
  immutable once posted.
- **Architecture list**: which targets have leaderboards and how each is measured
  (see Measurement tiers).
- **Reward pool and payout curve** (see Market design).

### Submission

- Rust source (published, or committed privately - see Privacy).
- Lean proof artifact filling the challenge's theorem hole, checked against the pinned
  toolchain versions (rustc, Charon, Aeneas, Lean, Mathlib) declared in the challenge.
- Build lockfile: fully reproducible build (pinned rustc, flags, dependencies).
  Compiler flags are chosen by the *challenge*, not the submitter, so the competition
  is about source-level algorithmic and layout skill, not flag roulette; a challenge
  may alternatively declare flags free and make them part of the submission.

### Verdict

A submission is **admitted** when the verification pipeline (deterministic,
re-runnable by anyone) passes:

1. Build reproducibly from lockfile.
2. Charon extracts LLBC; Aeneas produces the Lean model of the submitted code.
3. Lean checks that the submitted proof closes the challenge theorem against that
   model, with no `sorry`, no extra axioms beyond the challenge's allowed axiom set
   (checked with `#print axioms`).
4. Interface conformance and unsafe-policy lint.

Admission is objective and machine-checkable, so it can be verified on-chain the same
way Razor verifies proofs: either by a ZK proof of the Lean checker run (the
Razor/Yatima route), or optimistically with a fraud-proof window where anyone can
re-run the pipeline and slash a bad attestation.

## Measurement: the hard part

Correctness is machine-checkable; wall-clock performance is a physical measurement and
therefore an oracle problem. The Anvil is honest about this and offers three tiers with
different trust models. A challenge declares which tier(s) it uses per architecture.

### Tier 1 - Deterministic cost model (trust-minimized, default)

Score = instruction/cycle count in a pinned, deterministic simulator:

- CPU targets: a pinned cycle-approximate simulator (e.g. a fixed QEMU-plugin
  instruction counter, or gem5 with a frozen config) or static analysis (llvm-mca on
  the hot loop) - the challenge picks one and pins the binary by hash.
- wasm32: deterministic fuel metering - this target is *exactly* reproducible.
- The measurement is a pure function of (binary, harness, seed), so any dispute is
  resolved by re-execution. This tier can settle fully optimistically on-chain with
  fraud proofs, no committee needed.

Weakness: a cost model is a proxy; it can diverge from real silicon. That is a known,
declared tradeoff - like gas vs real cost in the EVM.

### Tier 2 - Attested hardware (TEE oracle)

Score = median wall-clock/cycles on real reference machines (one pinned SKU per
architecture, e.g. "AMD Zen 4 / Ryzen 9 7950X, fixed frequency, SMT off"), run inside
attested environments (AWS Nitro Enclaves / SEV-SNP) that sign (binary hash, harness
hash, result). Trust reduces to the TEE vendor plus the published runner image.
Multiple independent runner operators, take the median, slash outliers.

### Tier 3 - Staked benchmark committee (fallback)

For exotic hardware (GPUs, accelerators) where TEEs are unavailable: N staked
operators run the harness, commit results, reveal, median-settle, slash deviants
beyond a tolerance band. Weakest trust model, widest hardware coverage.

Anti-gaming rules common to all tiers:

- Input seeds are drawn from the randomness beacon of the settlement chain *after*
  submission commit, so implementations cannot special-case the benchmark inputs.
- The refinement theorem quantifies over the whole valid input domain, so an
  implementation cannot be correct only on benchmarked inputs.
- Harness pins thermal/frequency policy for Tier 2; runs interleave competitors
  round-robin to cancel drift.

## Market design

### Leaderboards first, money as a module

Each (challenge, architecture) pair has a leaderboard ordered by score among admitted
submissions. The protocol is free and permissionless: posting a challenge, submitting,
and holding a crown require no token. The leaderboard - public, verifiable,
per-architecture championship - is the base incentive, the same one that drives every
performance benchmark today, and it is enough for compute donors, researchers, and
labs who want a standing measure of their code-search capability.

Reward pools are an optional layer on top. Anyone who values a challenge (a crypto
library maintainer, a hardware vendor, a public-goods funder) can attach funds to it
as a price signal directing search compute toward what they actually need. When a
pool exists, it streams rather than pays lump-sum:

- **King-of-the-hill streaming**: the pool streams block-by-block to the current
  champion per architecture. Being beaten stops your stream. This keeps the incentive
  alive for continuous improvement instead of a one-shot race.
- **Improvement bounties**: a configurable share of the pool pays lump-sum on each
  strict improvement, scaled by relative gain (e.g. proportional to
  log(old_score/new_score)), so a 2x win pays more than a 0.1% shave.
- **Anti-sniping decay**: a minimum improvement threshold (e.g. >=1%) to take the
  crown prevents dust-improvement griefing.
- Sponsors and third parties can top up pools; per-architecture pools can be funded
  independently (an ARM vendor funds the aarch64 pool only).

### Roles

- **Sponsor**: posts spec + harness, optionally with a pool. Commercial users (crypto
  libraries, codecs, kernels for inference), public-goods funders, or anyone who wants
  a challenge on the frontier.
- **Implementer**: submits code + proof. Humans, teams, or AI search pipelines.
- **Spec author**: writing a good Lean spec is skilled work; the platform supports
  spec-authoring bounties (a Razor-style task: "formalize RFC 8439 in Lean") feeding
  directly into Anvil challenges. This is the natural bridge between the two platforms.
- **Runner/verifier operators**: run Tier 2/3 measurement and optimistic verification,
  staked and slashable.

### Fees

Fees apply only to the optional money layer, never to the registry itself: a protocol
fee on pool flows, private submission fees, and enterprise integrations (a crate
registry mirror serving champion implementations with their proofs - `cargo add` a
proven-fastest primitive).

## Privacy and front-running

Identical threat model to Razor, one difference: an implementation, unlike a proof, is
also the *product*, so sponsors usually want it published (that is what they are
paying for). Default: submissions are commit-reveal (hash on-chain at time T, reveal
within a window) to timestamp priority without leaking the code to competitors before
admission. Challenges may optionally allow ZK-private submissions where only the
binary hash, proof-of-admission, and score are public and the source is licensed to
the sponsor - the platform discourages but supports this.

## What lives on-chain

- Challenge registry: spec package hash, harness hash, toolchain pins, pool, payout
  curve, architecture list.
- Submission commitments, admission verdicts (ZK-verified or optimistic), scores per
  tier with attestation/committee signatures.
- Streaming payout logic and slashing.
- Content-addressed artifact store pointers (spec packages, sources, proofs, binaries) -
  bulk data on IPFS/Arweave, hashes on-chain, mirroring Razor's Lean package registry.

## Showcase challenge classes

Chosen so the spec is statable today and the performance market is real:

1. **Cryptographic primitives**: ChaCha20-Poly1305, SHA-256, Ed25519 field arithmetic,
   NTT for post-quantum schemes. Specs are pure functions over bitvectors; correctness
   is existential (carry bugs, reduction bugs); performance is commercially valuable
   per-architecture. This is the beachhead - Aeneas was literally built for this.
2. **Serialization/parsing**: protobuf/CBOR/RLP codecs. Spec: round-trip and
   canonical-form theorems. Parsing is where memory-safety CVEs live; a proven-fastest
   parser is a strong product.
3. **Compression**: spec is "decompress(compress x) = x" plus format conformance;
   score is throughput and ratio (multi-objective leaderboards supported: the
   challenge declares a scalarization).
4. **Numerical kernels**: GEMM, FFT over exact or fixed-point domains first
   (floating-point specs come later - the spec-side model of IEEE 754 exists in Lean,
   but refinement proofs through Aeneas for float code are research-grade; declared
   out of scope for v1).
5. **Chain infrastructure eating its own tail**: the ZK verifier, the Lean checker
   kernel, the simulator of Tier 1 itself - each can be posted as an Anvil challenge.

## Architecture (system components)

- **Spec SDK**: Lean library of challenge scaffolding - input domain combinators,
  refinement theorem templates, harness DSL.
- **Verification pipeline**: containerized deterministic pipeline
  (rustc → Charon → Aeneas → Lean check), pinned by hash in each challenge; the same
  container is what optimistic verifiers re-run and what the ZK track proves.
- **Runner network**: Tier 1 re-executors, Tier 2 TEE runner images, Tier 3 committee
  client.
- **Contracts**: registry, escrow/streaming, slashing, verdict verification.
- **Frontend**: challenge explorer, per-architecture leaderboards with score history,
  diff-view between champion implementations, proof browser shared with Razor.

## Roadmap

### Phase 1: Rails (months 1-6)
- Pipeline hardening: reproducible rustc/Charon/Aeneas/Lean container, pinned
  toolchain releases.
- Tier 1 only, wasm32 + x86-64 instruction-count. Optimistic settlement, single
  testnet.
- Three seed challenges from class 1 (crypto primitives), sponsor-funded.
- Success metric: one challenge with >=3 independent admitted submissions and a
  crown change.

### Phase 2: Real metal (months 7-12)
- Tier 2 TEE runners on pinned x86-64 and aarch64 SKUs.
- Streaming payouts, improvement bounties, commit-reveal.
- Spec-authoring bounty bridge to Razor.
- AI-implementer reference pipeline published (spec in, candidate+proof out) to seed
  the agent ecosystem.

### Phase 3: Frontier (months 13-24)
- Tier 3 committees for GPU targets; multi-objective leaderboards.
- ZK-verified admission (shared infrastructure with Razor's Yatima track).
- Verified crate registry product: `cargo`-installable champions with embedded proofs.
- Floating-point and concurrency research track.

## Honest open problems

- **Unsafe and intrinsics**: peak performance often needs `unsafe`, SIMD intrinsics,
  or asm, which Aeneas does not cover. v1 accepts the ceiling of safe Rust +
  autovectorization; the long-term answer is per-challenge trusted intrinsic models
  or binary-level tracks with a different verifier - explicitly research.
- **Cost-model gap**: Tier 1 crowns can differ from real-silicon crowns; challenges
  wanting real numbers must accept Tier 2 trust assumptions. There is no free lunch
  here and the platform says so.
- **Spec bugs**: a proof only transfers trust to the spec. Mitigation: the hole
  funnel ([FUNNEL.md](FUNNEL.md)) - spec-authoring bounties with challenge windows,
  executable-spec differential testing against reference implementations, staked
  ratification, and the exploit-as-audit rule (a submission that exploits a spec bug
  is paid, the spec is voided with lineage, and endorser stakes cover it).
- **Toolchain trust**: rustc, Charon, Aeneas, and the Lean kernel are all trusted.
  Pinning by hash makes the trust explicit and versioned; verified-compilation
  tracks can shrink it over time.

## Relationship to Satoshi's Razor

| | Razor | Anvil |
|---|---|---|
| Competition object | Proof of a statement | Implementation of a spec |
| Winning condition | First valid proof | Fastest admitted implementation, per architecture |
| Verification | Lean check (ZK-wrapped) | Lean refinement check (ZK-wrapped) + measurement oracle |
| Settlement | One-shot payout | Streaming king-of-the-hill |
| Output artifact | Theorem in the knowledge base | Verified library in the crate registry |
| Trust residue | Lean kernel | Lean kernel + toolchain + measurement tier |

Shared infrastructure: Lean package registry, proof verification (Yatima/ZK), artifact
storage, identity/KYC, frontend proof explorer. A Razor bounty ("formalize this RFC")
naturally feeds an Anvil challenge ("now implement it fast"), making the two platforms
a pipeline from open problem to proven theorem to production-grade verified code.
