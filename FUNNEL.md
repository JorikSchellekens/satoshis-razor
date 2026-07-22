# The Sorry Funnel

How Satoshi's Razor organizes the proposal, formalization, and refinement of sorries -
including conjectures and statements that are not yet machine-checkable.

The registry's guarantee - a submission checks or it doesn't - only applies once a
problem is a pinned Lean statement. Everything upstream of that point (is this
conjecture worth stating? does this Lean statement faithfully capture the informal
conjecture?) is not machine-checkable and never fully will be. The funnel is the
protocol's answer: a pipeline that moves problems from informal idea to pinned sorry,
using machine-checkable evidence wherever it exists and shrinking the social judgment
to its smallest possible form where it doesn't.

## Design principle

**Fidelity cannot be verified, but weakness is self-exposing and convergence is
checkable.**

Statement fidelity - whether a formalization means what the informal conjecture
meant - is inherently social, and no incentive scheme removes the human judgment at
the end. What the funnel does instead:

- A statement that is *too weak* (forgot a condition, admits a degenerate case) is
  usually trivially provable, and ordinary proof search finds the trivial proof
  almost immediately - no dedicated auditor needed. The registry records the facts
  and nothing more: proven, kernel check took N milliseconds, clump of weight 1,
  outside the dominant clump. It cannot know - and does not rule on - whether that
  means mistranslation or a genuinely easy problem. A proven weight-1 statement
  that checked in milliseconds tells its own story.
- A statement that is *too strong or subtly shifted* cannot be caught that way. The
  defense is **independent convergence**: equivalent formalizations written by
  people who did not see each other's work. Equivalence is machine-checkable, so
  this evidence is trustless; independence is the only assumption.
- Value is assigned two ways, neither requiring adjudication. **Curation** is the
  primary one: costless, attributed, weighted by the curator's verified work, and
  publicly scoreable in hindsight. **Bounties** are the optional edge: credits
  attached to one exact pinned statement by a funder confident that a proof of that
  precise wording is worth something to them. The first admitted proof takes the
  bounty, degenerate proofs included - the fidelity risk sits with the funder, who
  had the clump evidence in front of them when they spent.
- An immutable lineage keeps every mistake, exposure, and correction on the log, so
  priority and attribution survive corrections.

## Lifecycle

```
proposed -> candidate statements (each with a gloss)
                 |  equivalence proofs (convergence)
                 v
              clumps  ->  dominant clump  ->  pinned sorries  ->  solved / disproven
                 |
                 v
   wrong wording (facts on the log) -> supersession marks -> attention moves on
```

Statements are never mutated and never closed by anyone's word. When a wording is
wrong, attributed supersession marks point from it to the better sorry, with
explicit lineage, so priority and attribution records survive every correction.

## Stage 1: Proposal (informal, off the trust-critical path)

Anything can be proposed: a natural-language conjecture, a literature reference,
"formalize RFC 8439", "we need a formal theory of X". This is a staging area with
discussion threads - arXiv-meets-Zulip - deliberately cheap and permissive, because
filtering happens downstream. The only permanent artifact is a content hash and
timestamp, which establishes credit for the *idea* itself. Money never attaches to a
proposal - there is no precise wording yet to buy a proof of. Curation is how
importance is assigned at this stage.

## Stage 2: Candidate statements, glosses, and clumps

Turning an informal proposal into a Lean statement is skilled work and a first-class
contribution. Anyone may attach a **candidate statement** to a proposal: a Lean
statement filed together with the author's own plain-language reading of it - the
**gloss**. The gloss decomposes the fidelity question into two easier ones:
does the gloss say what the proposal says (plain language against plain language),
and does the Lean say what the gloss says (a local, careful reading).

### Machine-checkable certificates
Genuinely checkable evidence, with standard templates:
- **Non-vacuity**: proof that the hypothesis set is inhabited.
- **Non-triviality**: the statement does not follow in a few lines from the pinned
  library, and its negation is not trivially provable.
- **Instance checks**: `#eval` / decidability checks on small cases; counterexample
  search coming up empty.
- **Expected corollaries**: proofs that the statement implies the things the informal
  conjecture is known to imply.

### Convergence and clumps (the trust mechanism)
A machine-checked equivalence proof between two candidate statements is an edge;
the connected components are **clumps**. A clump's **weight** counts its distinct
authors - an independence proxy, since fifty equivalent statements from one source
are one voice. Two people independently producing equivalent formalizations is the
strongest evidence available that both are faithful, so the protocol treats
duplicate formalization as a feature: a proposal can fund several statement efforts
plus the equivalence proofs between their outputs.

Independence is the load-bearing assumption, and the registry turns as much of
it as possible into recorded fact (see "Reading windows" below): a statement
can be filed *sealed* - a hash commitment first, the reveal later - and two
statements each sealed before the other was revealed were provably written
blind to each other. A clump therefore carries two counts: its **weight**
(distinct authors - claimed independence) and its **written-blind count** (the
largest set of distinct authors whose statements are pairwise sealed before one
another's reveals - proven independence). Definitional diversity - clump
members built on different underlying definitions - counts for more than raw
head-count. What sealing cannot rule out is off-platform collusion, so with no
money at stake, sybil identities cost nothing to create, and weight should
count authors with verified work histories - identities that were expensive to
build the only way this system respects, by doing checkable work. The same
applies to curation weight. This residual reliance on reputation is an
assumption, and it is named here rather than hidden.

One-way **implication proofs** are recorded too: a proof of A → B with no converse
mechanically exposes B as no stronger than A, and often as strictly weaker. This
orders competing readings without any adjudication.

### Reading windows and sealed readings

Convergence is only as strong as the independence behind it, and by default
nothing produces independence: the second formalizer can simply read the first
candidate statement and paraphrase it. The registry's answer is sealing, the
same commit-reveal scheme private proof submissions already use, applied to
statements:

- A **reading window** (`razor round`) is a dated invitation on a proposal:
  seal your reading by one date, reveal it by another. The window is a
  coordination signal, not a gate - the registry never enforces it, and a late
  seal or reveal simply carries its own timestamps.
- A **sealed reading** (`razor seal-statement`) puts sha256(statement file ‖
  salt) on the log now; the file and salt stay on the author's machine. The
  reveal (`razor reveal-statement`) is checked against the commitment before
  it is appended, and the statement enters the funnel as an ordinary candidate
  carrying its seal's provenance.
- The trust fact is **pairwise mutual blindness**, computed from event order
  alone: statements X and Y are mutually blind when X was sealed before Y was
  revealed *and* Y was sealed before X was revealed - neither author could
  have seen the other's Lean. An unsealed statement's filing is its reveal, so
  two unsealed statements are never provably blind; a sealed statement can be
  provably blind even to statements filed later. No window membership is
  consulted: the dates exist to coordinate people, the math reads the log.

When mutually blind statements later prove equivalent, the funnel's strongest
evidence gets an upgrade: independent convergence where the independence is
itself on the record. Sealing cannot prevent authors from colluding off the
log, which is why reputation-weighted counting remains; it removes the default
contamination channel - reading each other's filings - entirely.

### Bridge sorries

An equivalence between two candidate statements can be recorded two ways. The
direct way (`razor converge`) attaches a named equivalence declaration as an
edge. The stronger way pins the obligation itself: `razor bridge` registers a
**bridge sorry** whose statement is composed mechanically from the two
declarations - `(a's decl) ↔ (b's decl)` - exactly as split glue is composed
from pinned types, so there is nothing for the filer to get subtly wrong. The
bridge is then an ordinary sorry: solved through submit/verify (the equivalence
is kernel-checked, which `converge` alone does not do), attributed to whoever
proves it, curatable, and fundable - a proposer who wants convergence evidence
can put a bounty on the bridge. An admitted bridge proof merges the two
statements' clumps.

Equivalence work is a first-class contribution here, deliberately: between two
independently written formalizations the bridge proof is often harder than
either statement, and it is exactly the labor that turns a pile of readings
into a dominant clump.

### Dominance
The unique heaviest clump with **at least two independent members** is
the proposal's **dominant** clump. Dominance carries no payment and triggers
nothing; it is an epistemic label - the reading the community has converged on -
and the signal a rational funder waits for before putting a bounty on any
statement. A singleton clump is never dominant: one author's word, however good,
is exactly the thing this design refuses to bless.

### Curation (how value is assigned without money)
Anyone may **curate** a proposal, statement, or sorry: a signed, timestamped,
costless mark that this is worth working on, with a note saying why. A curation's
weight grows with the curator's verified work on the record, so the taste of
people who have actually solved things counts for more - and because picks are
public and permanent, a curator's judgment accrues a track record exactly the way
a solver's proofs do. This is the Millennium-Problems model made permissionless:
lists compete, and the registry does not pick winners.

### Bounties (money on exact wordings, caveat emptor)
Anyone confident that a proof of one *exact pinned statement* is valuable to them
may attach credits to it. The rule is deliberately absolute: the first admitted
proof of the literal statement takes the bounty - degenerate and trivial proofs
included - with no adjudication and no refunds. There is nothing to adjudicate,
because the funder was never promised the proposal's *meaning*; they bought the
statement's *wording*, with the clump weight, glosses, certificates, and
implication order in front of them when they chose it. A bounty eaten by a
two-line proof is the system working: the loss lands precisely on the misplaced
confidence, the check time is on the log, and everyone else learns which wordings
not to trust - and to wait for convergence before spending.

### Weak statements in practice
When a too-weak statement is trivially proven, what goes on the log is what
happened, nothing more: the
proof, its kernel-check cost, and (via equivalence transfer) that the whole clump
is proven. No collapse verdict exists in the schema, and nothing closes: anyone
may file an attributed **supersession mark** pointing at the better sorry
(see "Superseding, precisely"), and readers weigh the marks. What the author of
the weak statement loses is reputational and visible - their gloss stands on the
log next to a statement whose trivial proof is also on the log.

## Stage 3: Splits (partial proofs, trustlessly shared)

In everyday Lean work, partial progress on a theorem is a file that proves it
with `sorry` in place of the missing pieces. A **split** is that artifact made
public and load-bearing. Registering one names the missing pieces as child
sorries and creates one more sorry - the **glue** - whose pinned statement the
registry composes mechanically from types already on the record:

    (child 1) → (child 2) → ... → (child n) → parent

The splitter proves the glue through the ordinary submit/verify path. Because
the glue takes the child *statements* as hypotheses, it is provable - and
kernel-checked - while every child is still open. That is the entire trust
story: an admitted glue is a machine-checked fact that solving the children
finishes the parent, so splits are permissionless. Anyone can split anyone's
sorry, blueprint-style (the structure of large formalization projects like the
PFR blueprint and the FLT project).

- Several splits of one parent coexist, each a first-class object with its own
  author, note, glue, and children. A sorry can serve as a child in any number
  of splits. The registry derives per-split facts: how many children are
  solved, whether the glue is admitted, and whether the split is *complete*
  (glue and all children admitted - the parent is then provable by
  composition, and substituting the child proofs for the sorries makes the
  original file compile).
- A split is never edited or deleted. Refactoring a decomposition - the cut
  was wrong, a child's definitions need changing - means registering another
  split alongside it, exactly as weak statements are superseded rather than
  rewritten. The old glue remains a true theorem whether or not anyone follows
  its plan; children unique to an abandoned split remain honest open problems
  or quietly attract no further work. The live registry carries a real
  example: FLT's classical reduction (exponent 4 + odd primes) and the FLT
  project's actual plan (3, 4, and primes >= 5) are two splits of the same
  parent sorry, both with kernel-checked glue.
- Solving a child is a first-class, attributed contribution, and curation and
  bounties attach to children like any other sorry.

## Stage 4: Prioritization (signals, not a canonical ranking)

The protocol stays neutral: it exposes signals and lets curation compete on top.

- **Curation weight**: who flagged this as important, weighted by their verified
  work - the primary importance signal.
- **Demand**: bounties on exact statements are the honest price signal for what
  someone concretely values.
- **Convergence weight**: how many independent formalizations agree - the trust
  signal, directly visible per clump.
- **Structural centrality**: computed from the dependency graph - how many open sorries
  does this sorry unblock (the "most-wanted lemma" metric).
- **Revealed difficulty**: attempt telemetry. Agents log failed attempts; sorries
  accumulate an Elo-like rating - a sorry that survives many strong attempts rates
  harder, and agents that crack highly-rated sorries gain rating. This produces an
  emergent difficulty map of open mathematics, which is itself a valuable artifact
  and a far better AI benchmark than any static problem set.
- **Editorial curation**: anyone can publish a signed, reputation-bearing "frontier
  list" (the Millennium Problems model, made permissionless). Compute donors
  subscribe to curators they trust. Curation competes; the registry does not pick
  winners.

## Closure semantics

- **Disproof is a first-class solution.** Proving the negation closes a conjecture
  sorry with full credit and payout. A conjecture is a question, not a side to bet on.
- **Solved** sorries become versioned packages others build on (see README).
- **Superseded** wordings point to their successors via weighted marks; the
  sorry itself never closes and nothing is deleted.
- **Closed by citation.** A sorry whose statement is found to already exist in a
  recognized prior corpus (see below) closes with a citation instead of a proof.

## Superseding, precisely

The right wording wins by the same signals that build trust - convergence and
curation - never by anyone closing anything:

1. Corrected statements accumulate under the proposal like any others, converge,
   and (usually) form the new dominant clump. The facts against the old wording
   (proven trivially, weight 1, milliseconds to check) are already on the log.
2. Anyone may file an attributed `supersede` event: a **supersession mark** saying
   sorry X is better stated by sorry Y, with the reason. Marks are weighted by the
   filer's verified work, exactly like curations, and several marks - even
   disagreeing ones - can coexist. A mark changes no status: the old sorry stays
   provable, its admitted proofs and payouts stand, and the log is append-only.
3. What a mark changes is what readers see: the frontier shows who considers a
   wording superseded and by what, sorts the sorry down in proportion to the
   weight against it, and links forward to the replacement.
4. The new sorry's **lineage** is the count of earlier wordings whose marks point
   at it. Lineage 2 means "the third attempt at stating this problem" - which is
   evidence the statement has been tested, not a mark against it.

Priority and attribution survive the chain: work done against a superseded wording
stays on the record, including the trivial proofs that motivated each mark.

## Statement rot and repinning

Superseding covers the case where a wording was *wrong*. A pinned statement can
also become *stale* while staying right: the library it is written against
renames a definition, restructures a namespace, or respells the same Prop. This
is not hypothetical - Mathlib refactors continuously, and a registry that pins
exact statements without a migration story would slowly fill with sorries nobody
can read or target.

The answer is `razor repin`, and it is held to the same standard as everything
else: **a sorry migrates to a new wording only if a proof that the two wordings
are equivalent kernel-checks.** The CLI composes the obligation mechanically -
`new ↔ old` - so there is nothing for the migrator to get subtly wrong, and it
refuses the event if the equivalence does not check.

What this preserves:

- **Prior admissions stay valid.** A proof admitted against the old wording is
  still a proof of the new one, because the equivalence is itself a checked
  theorem. Nothing is re-judged; truth transfers along the proven iff.
- **The full history stays on the log.** The old wording, the new wording, and
  the equivalence declaration are all recorded; a reader auditing a five-year-old
  admission can replay the exact statement it was checked against and walk the
  equivalence chain forward.
- **Priority survives churn.** The sorry keeps its identity, its bounty pool, its
  splits, and its submissions across any number of repins.

Repinning handles respellings; it cannot handle a definition that genuinely
changed meaning (no equivalence exists to prove). That case is a new wording
under the same proposal - the supersession machinery above - which is exactly
the distinction the kernel enforces: if you can prove the iff, it is the same
problem; if you cannot, it is a different one.

The continuous-integration build elaborates every pinned Mathlib statement
against the pinned Mathlib version, so rot is detected the day it happens, not
the day a solver hits it.

## Units: credits

Bounties and payouts are denominated in **credits**, a hypothetical accounting
unit. The registry maintains the ledger and moves no real money; there is no
token. The incentive analysis in this document depends only on structure - money
attaches to exact wordings, fidelity risk on the funder, importance carried by
curation - not on what backs the unit. A deployment may bind credits to
reputation, grant funding, or a currency; the funnel is agnostic. A deployment
may equally run with no credits at all: curation, attribution, and priority are
the primary rewards, and they need no backing.

## Prior corpora

The registry does not restate work that is already machine-checked elsewhere. A
`recognize-corpus` event records an external verified corpus - Mathlib is the
canonical example - together with sourced, dated statistics. Its contents count as
solved:

- New sorries should state what the corpora do not contain; that boundary is the
  frontier.
- A sorry later found to duplicate a corpus result is closed by citation.
- Statements may be written directly with a corpus's definitions: a sorry registered
  with the Mathlib environment pins a statement whose definitions are Mathlib's
  own, which is itself a fidelity defense - definitions used in thousands of
  existing theorems are much harder to get silently wrong than fresh ones.

## The deepest case: definition sorries

Sometimes a conjecture cannot even be *stated* because the library lacks the objects
(the perfectoid-spaces situation). Definition sorries are the hardest fidelity problem:
a wrong definition can silently poison every theorem built on it. The available
machinery is the same two tools, applied harder:

- **Independent convergence**: parallel definitions plus machine-checked equivalence.
- **Characterization certificates**: machine-checked proofs that the new definition
  behaves exactly as the literature demands - it reduces to the classical notion in
  the special case, satisfies the standard axioms, admits the known examples and
  excludes the known non-examples. A definition that provably behaves correctly in
  every way the literature can articulate is very hard to get wrong silently.

Definition sorries carry longer reading windows and the strongest expectation of
convergence evidence, proportional to their blast radius.

## Application to Satoshi's Anvil

The funnel applies unchanged to [Anvil](ANVIL.md) challenges, where the fidelity
question is "does this Lean spec capture the intended behavior?" (e.g. does this
formalization actually match RFC 8439). Anvil's equivalents:

- Statement sorries become **spec-authoring bounties**; candidate specs run
  differential tests against reference implementations during the reading window -
  an executable-spec certificate unavailable to pure mathematics.
- The bounty rule is Anvil's spec-bug policy: a challenge pool pays for the pinned
  executable spec exactly as written, so an implementation that wins by exploiting
  a spec bug is paid, the bug becomes public, and supersession marks point the
  spec at its corrected successor - the sponsor funded the wording they endorsed.
- Decomposition maps to layered challenges: a verified primitive becomes a pinned
  dependency of a larger spec.
