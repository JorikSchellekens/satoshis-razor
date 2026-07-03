#!/usr/bin/env bash
# End-to-end demonstration of Satoshi's Razor + Anvil + the hole funnel.
# Every step below is a real registry event; every verification is a real
# Lean kernel check; every score is a real measurement.
set -euo pipefail
cd "$(dirname "$0")"

RAZOR=./target/release/razor
step() { printf '\n\033[1;36m▸ %s\033[0m\n' "$*"; }

command -v python3 >/dev/null 2>&1 || { echo "python3 is required (decodes the zk prover's output)" >&2; exit 1; }

step "Build everything (Lean package, registry, harness, wasm submissions)"
(cd lean && lake build 2>&1 | tail -1)
cargo build --release 2>&1 | tail -1
cargo build --release --target wasm32-unknown-unknown \
  -p popcount-naive -p popcount-swar -p sum-loop -p sum-closed -p sort8-bubble -p sort8-network -p evm-ref -p evm-tos 2>&1 | tail -1

step "Fresh registry"
rm -rf registry/data site/data.json lean/Razor/Private lean/Razor/Submissions

# ─────────────────────────────────────────────────────────────────────
step "PROLOGUE - Participants claim their handles"
# ─────────────────────────────────────────────────────────────────────
$RAZOR account new --handle alice --display "Alice" --about "writes formal statements, then defends them"
$RAZOR account new --handle bob --display "Bob" --about "proof search, mostly by hand"
$RAZOR account new --handle mallory --display "Mallory" --about "reads specifications very literally"
$RAZOR account new --handle heidi --display "Heidi" --about "solves subgoals"
$RAZOR account new --handle judy --display "Judy" --about "makes bits go fast" --github judy-razor-demo
$RAZOR account new --handle leo --display "Leo" --about "interpreter internals"
$RAZOR account new --handle peggy --display "Peggy" --about "proves things without showing them"

# ─────────────────────────────────────────────────────────────────────
step "ACT I - Simple proofs: the basic loop (propose → hole → solve → verify → payout)"
# ─────────────────────────────────────────────────────────────────────

$RAZOR propose --id PRP-001 --author alice \
  --title "Gauss sum formula" \
  --body "The sum of the first n naturals is n(n+1)/2. Stated multiplicatively to avoid division."
$RAZOR hole --id RZR-001 --proposal PRP-001 \
  --title "2 * sumTo n = n * (n + 1)" \
  --lean-type "∀ n : Nat, 2 * Razor.sumTo n = n * (n + 1)"
$RAZOR fund --target RZR-001 --amount 500 --funder math-dao
$RAZOR submit --id SUB-001 --hole RZR-001 --solver bob --decl Razor.gauss
$RAZOR verify --submission SUB-001

$RAZOR propose --id PRP-002 --author alice \
  --title "Reversal is an involution" \
  --body "Reversing a list twice gives back the list, for a from-scratch accumulator reversal."
$RAZOR hole --id RZR-002 --proposal PRP-002 \
  --title "rev (rev l) = l" \
  --lean-type "∀ {α : Type} (l : List α), Razor.rev (Razor.rev l) = l"
$RAZOR fund --target RZR-002 --amount 300 --funder math-dao
$RAZOR submit --id SUB-002 --hole RZR-002 --solver carol --decl Razor.rev_rev
$RAZOR verify --submission SUB-002

# ─────────────────────────────────────────────────────────────────────
step "ACT II - The worked example: a weak statement is funded, trivially proven, and replaced"
# ─────────────────────────────────────────────────────────────────────

$RAZOR propose --id PRP-100 --author alice \
  --title "Provably correct sorting" \
  --body "There is a sorting function for lists of naturals, with a machine-checked correctness proof."
$RAZOR curate --curator sortware-inc --target PRP-100 \
  --note "we ship a sort kernel; a machine-checked one is worth real money to us"

step "dave's formalization forgets the permutation clause (his gloss says more than his Lean does)"
$RAZOR formalize --id STM-101 --proposal PRP-100 --author dave \
  --decl Razor.Sorting.V1Statement \
  --gloss "there is a function that sorts every list" \
  --notes "∃ f, ∀ l, SortedChain (f l) - output must be sorted. (What else could go wrong?)"
$RAZOR hole --id RZR-103 --proposal PRP-100 --statement STM-101 \
  --title "A sorting function exists (v1)" \
  --lean-type "Razor.Sorting.V1Statement"

step "sortware-inc trusts dave's gloss and puts a bounty on his exact statement"
$RAZOR fund --target RZR-103 --amount 2000 --funder sortware-inc
# A bounty attaches to one pinned statement, never to the proposal. The
# funder pays for the statement as written - the registry warned them that
# STM-101 is a clump of one with no convergence evidence.

step "mallory's ordinary proof search finds the two-line proof - it verifies, and it pays"
$RAZOR submit --id SUB-103 --hole RZR-103 --solver mallory --decl Razor.Sorting.v1_exploited
$RAZOR verify --submission SUB-103
# mallory takes the 2,000. Correctly: sortware-inc funded the literal
# statement, and the literal statement is trivially true. The kernel check
# took milliseconds and that fact is on the log. The loss lands exactly on
# the party who chose to trust an unconverged statement, and the lesson is
# public: fund a statement after a clump forms around it, not before.

step "Convergence: alice and bob formalize the corrected statement independently"
$RAZOR formalize --id STM-102A --proposal PRP-100 --author alice \
  --decl Razor.Sorting.V2Statement \
  --gloss "the output is sorted AND is a rearrangement of the input; sortedness as an inductive chain" \
  --notes "∃ f, ∀ l, SortedChain (f l) ∧ Perm l (f l)"
$RAZOR formalize --id STM-102B --proposal PRP-100 --author bob \
  --decl Razor.Sorting.V2StatementPairs \
  --gloss "same reading, sortedness written as: every earlier element ≤ every later one" \
  --notes "written without seeing STM-102A (indexed SortedPairs definition)"
$RAZOR certify --statement STM-102A --kind non-vacuity \
  --decl Razor.Certificates.sorted_nonvacuous \
  --notes "sorted lists of length ≥ 3 exist"
$RAZOR certify --statement STM-102A --kind falsifiability \
  --decl Razor.Certificates.sorted_falsifiable \
  --notes "unsorted lists exist: the predicate is not vacuous"
$RAZOR converge --a STM-102A --b STM-102B --decl Razor.Sorting.v2_convergence

step "The implication order: the clump's statement is strictly stronger than dave's"
$RAZOR implies --a STM-102A --b STM-101 --decl Razor.Sorting.v2_implies_v1
# v2 → v1 is machine-checked; the converse would need a real sorting function
# to fall out of the empty-list function, so STM-101 sits strictly below.

step "v2: the hole for the dominant clump (weight 2 - alice and bob independently)"
$RAZOR hole --id RZR-103v2 --proposal PRP-100 --statement STM-102A \
  --title "A correct sorting function exists (v2)" \
  --lean-type "Razor.Sorting.V2Statement"

step "alice and bob mark dave's hole as superseded - weighted opinion, nothing closes"
# A supersession mark is a public, attributed pointer from one wording to a
# better one. RZR-103 stays exactly as provable as before (its proof and
# payout stand); readers weigh the marks by who filed them.
$RAZOR supersede --hole RZR-103 --by alice --replacement RZR-103v2 \
  --note "forgets to require the output be a rearrangement of the input; RZR-103v2 states it"
$RAZOR supersede --hole RZR-103 --by bob --replacement RZR-103v2 \
  --note "satisfied by the empty-list function (SUB-103); the clump's wording is the real problem"

step "sortware-inc funds again - this time a statement two people independently converged on"
$RAZOR fund --target RZR-103v2 --amount 2000 --funder sortware-inc

step "A split: partial progress with the gaps as named child holes"
# In a Lean file this is a proof of the parent with two sorries. Registered,
# the two sorries become child holes and the surrounding proof becomes the
# glue hole, whose statement the CLI composes mechanically from the pinned
# types: (child a) → (child b) → parent.
$RAZOR hole --id RZR-103a --proposal PRP-100 \
  --title "insert preserves sortedness (child a)" \
  --lean-type "∀ (x : Nat) {l : List Nat}, Razor.Sorting.SortedChain l → Razor.Sorting.SortedChain (Razor.Sorting.insert x l)"
$RAZOR hole --id RZR-103b --proposal PRP-100 \
  --title "insert adds exactly one occurrence (child b)" \
  --lean-type "∀ (a x : Nat) (l : List Nat), Razor.Sorting.count a (Razor.Sorting.insert x l) = (if x = a then 1 else 0) + Razor.Sorting.count a l"
$RAZOR split --id DEC-103 --parent RZR-103v2 --author grace \
  --child RZR-103a --child RZR-103b \
  --note "insertion sort; the two lemmas the induction needs are left open"
$RAZOR submit --id SUB-103g --hole DEC-103-glue --solver grace --decl Razor.Sorting.glue_v2
$RAZOR verify --submission SUB-103g

step "Subgoals solved independently, then v2 closes - and the bounty pays"
$RAZOR submit --id SUB-103a --hole RZR-103a --solver heidi --decl Razor.Sorting.insert_sorted
$RAZOR verify --submission SUB-103a
$RAZOR submit --id SUB-103b --hole RZR-103b --solver ivan --decl Razor.Sorting.insert_count
$RAZOR verify --submission SUB-103b
$RAZOR submit --id SUB-103v2 --hole RZR-103v2 --solver heidi --decl Razor.Sorting.v2_solution
$RAZOR verify --submission SUB-103v2
# First admitted proof of the pinned statement takes the bounty - same rule
# that paid mallory, now paying for the intended theorem, because this time
# the funder waited for convergence before trusting the wording.

step "The open frontier: registered, funded, unsolved (a sorried submission bounces)"
$RAZOR hole --id RZR-104 --proposal PRP-100 \
  --title "merge preserves sortedness" \
  --lean-type "∀ {l₁ l₂ : List Nat}, Razor.Sorting.SortedChain l₁ → Razor.Sorting.SortedChain l₂ → Razor.Sorting.SortedChain (Razor.Sorting.merge l₁ l₂)"
$RAZOR hole --id RZR-105 --proposal PRP-100 \
  --title "merge preserves counts" \
  --lean-type "∀ (a : Nat) (l₁ l₂ : List Nat), Razor.Sorting.count a (Razor.Sorting.merge l₁ l₂) = Razor.Sorting.count a l₁ + Razor.Sorting.count a l₂"
$RAZOR hole --id RZR-106 --proposal PRP-100 \
  --title "insertion sort is idempotent" \
  --lean-type "∀ (l : List Nat), Razor.Sorting.isort (Razor.Sorting.isort l) = Razor.Sorting.isort l"
$RAZOR fund --target RZR-104 --amount 800 --funder mergesort-fans
$RAZOR curate --curator alice --target RZR-104 \
  --note "merge is the next structural lemma; everything list-shaped goes through it"
$RAZOR curate --curator heidi --target RZR-106 \
  --note "idempotence is a good first hole for newcomers"
$RAZOR submit --id SUB-104x --hole RZR-104 --solver mallory --decl Razor.Sorting.merge_sorted
$RAZOR verify --submission SUB-104x

# ─────────────────────────────────────────────────────────────────────
step "Statement rot: the wording refactors, the hole survives (repin)"
# ─────────────────────────────────────────────────────────────────────
# A pinned statement can rot: a style refactor respells the same Prop.
# RZR-104's original wording binds its lists implicitly; the refactored
# wording binds them explicitly. `razor repin` migrates the hole only
# because the equivalence of the two wordings kernel-checks
# (Razor.Sorting.merge_sorted_binder_equiv). Mallory's rejection above and
# every other verdict stay valid: the old wording, the new wording, and
# the equivalence proof all remain on the log.
$RAZOR repin --hole RZR-104 --author grace \
  --lean-type "∀ (l₁ l₂ : List Nat), Razor.Sorting.SortedChain l₁ → Razor.Sorting.SortedChain l₂ → Razor.Sorting.SortedChain (Razor.Sorting.merge l₁ l₂)" \
  --equiv-decl Razor.Sorting.merge_sorted_binder_equiv \
  --note "binder-style refactor: implicit list arguments made explicit; same problem, kernel-checked equivalence"

# ─────────────────────────────────────────────────────────────────────
step "A solve arrives as a single .lean file (no package surgery)"
# ─────────────────────────────────────────────────────────────────────
# judy proves RZR-105 in one file on her own machine. `razor submit
# --file` installs it into the hole's package as a fresh module and
# builds it; she never touches the package layout.
JUDY_DIR=$(mktemp -d)
cat > "$JUDY_DIR/merge_count.lean" <<'LEAN'
import Razor

namespace Razor.Demo.MergeCount

open Razor.Sorting

theorem merge_count_solution (a : Nat) (l₁ l₂ : List Nat) :
    count a (merge l₁ l₂) = count a l₁ + count a l₂ := by
  induction l₁ generalizing l₂ with
  | nil => simp [merge, count]
  | cons x xs ih =>
    induction l₂ with
    | nil => simp [merge, count]
    | cons y ys ih₂ =>
      simp only [merge]
      split
      · simp only [count, ih (y :: ys)]
        omega
      · simp only [count] at ih₂ ⊢
        omega

end Razor.Demo.MergeCount
LEAN
$RAZOR submit --id SUB-105 --hole RZR-105 --solver judy \
  --decl Razor.Demo.MergeCount.merge_count_solution \
  --file "$JUDY_DIR/merge_count.lean"
$RAZOR verify --submission SUB-105

step "An admitted proof is citable: seq + log hash pin the fact"
$RAZOR cite SUB-105

step "Anyone can independently recheck the claim - one command, nothing written"
# This is what a "machine X solved open problem Y" announcement reduces to:
# replay the kernel check against the pinned statement, audit the signature
# on the claim, compare with the recorded verdict.
$RAZOR recheck --submission SUB-105

step "An admitted proof is carried to its home library, and the log records where it landed"
# Without --pr this drafts a contribution file with the proof source and a
# provenance header; with --pr it records the landing. The registry counts
# upstreamed proofs, not admitted ones, as its measure of usefulness.
$RAZOR upstream --hole RZR-105 --out "$JUDY_DIR/upstream-draft.lean"
$RAZOR upstream --hole RZR-105 --by judy \
  --pr "https://github.com/example/library/pull/104" \
  --note "demo: where the merge_count proof would land"

step "The frontier is exportable as proving targets (miniF2F-shaped JSONL)"
$RAZOR export-benchmark | head -2

# ─────────────────────────────────────────────────────────────────────
step "ACT III - The Anvil: verified implementations compete on speed"
# ─────────────────────────────────────────────────────────────────────

step "Benchmark rigs: the deterministic referee, plus hardware the sponsor brings"
$RAZOR rig --id wasm-referee --owner protocol --arch wasm32 --tier wasm-fuel \
  --note "wasmtime fuel metering - deterministic, anyone can re-run and settle disputes"
$RAZOR rig --id m4-station --owner bitboard-labs --arch aarch64-apple-m --tier native \
  --note "Apple M-series box brought by the sponsor; scores signed by its owner"

step "ANV-001 popcount: SWAR submission, admission by SAT-settled refinement proof"
$RAZOR challenge --id ANV-001 --title "popcount(u64)" --spec-impl popcount-naive \
  --obligation "∀ x : BitVec 64, model x = Razor.Anvil.popNaive x"
$RAZOR fund --target ANV-001 --amount 5000 --funder bitboard-labs
$RAZOR fund --target ANV-001 --amount 3000 --funder bitboard-labs --arch aarch64-apple-m
$RAZOR anvil-submit --id ANV-001-ref --challenge ANV-001 --impl popcount-naive \
  --solver spec-author --proof-decl ""
$RAZOR hole --id ANV-001-SWAR-PROOF \
  --title "popcount-swar refines the popcount spec" \
  --lean-type "∀ x : BitVec 64, Razor.Anvil.popSwar x = Razor.Anvil.popNaive x" \
  --allow-axiom "bv_decide" --allow-axiom "Lean.ofReduceBool"
$RAZOR submit --id SUB-ANV1 --hole ANV-001-SWAR-PROOF --solver judy --decl Razor.Anvil.swar_refines
$RAZOR verify --submission SUB-ANV1
$RAZOR anvil-submit --id ANV-001-swar --challenge ANV-001 --impl popcount-swar \
  --solver judy --proof-decl Razor.Anvil.swar_refines --refinement-hole ANV-001-SWAR-PROOF
$RAZOR bench --challenge ANV-001 --iters 20000 --rig wasm-referee
$RAZOR bench --challenge ANV-001 --iters 20000 --rig m4-station

step "ANV-002 sum(1..n): closed form beats the loop, admission by algebraic proof"
$RAZOR challenge --id ANV-002 --title "sum(1..n)" --spec-impl sum-loop \
  --obligation "∀ n : Nat, model n = Razor.Anvil.sumLoopModel n (valid: n < 2^32)"
$RAZOR fund --target ANV-002 --amount 5000 --funder gauss-capital
$RAZOR anvil-submit --id ANV-002-ref --challenge ANV-002 --impl sum-loop \
  --solver spec-author --proof-decl ""
$RAZOR hole --id ANV-002-CLOSED-PROOF \
  --title "sum-closed refines the sum spec" \
  --lean-type "∀ n : Nat, Razor.Anvil.sumClosedModel n = Razor.Anvil.sumLoopModel n"
$RAZOR submit --id SUB-ANV2 --hole ANV-002-CLOSED-PROOF --solver kevin --decl Razor.Anvil.closed_refines
$RAZOR verify --submission SUB-ANV2
$RAZOR anvil-submit --id ANV-002-closed --challenge ANV-002 --impl sum-closed \
  --solver kevin --proof-decl Razor.Anvil.closed_refines --refinement-hole ANV-002-CLOSED-PROOF
$RAZOR bench --challenge ANV-002 --iters 20000 --rig wasm-referee
$RAZOR bench --challenge ANV-002 --iters 20000 --rig m4-station

step "ANV-003 sort 8 bytes: a 19-comparator network, admission by SAT - nobody has to see why it sorts"
$RAZOR challenge --id ANV-003 --title "sort the 8 bytes of a u64" --spec-impl sort8-bubble \
  --obligation "∀ x : BitVec 64, model x = Razor.Anvil.sortBubble x"
$RAZOR fund --target ANV-003 --amount 4000 --funder bitboard-labs
$RAZOR anvil-submit --id ANV-003-ref --challenge ANV-003 --impl sort8-bubble \
  --solver spec-author --proof-decl ""
$RAZOR hole --id ANV-003-NET-PROOF \
  --title "the 19-comparator sorting network refines the bubble-sort spec" \
  --lean-type "∀ x : BitVec 64, Razor.Anvil.sortNetwork x = Razor.Anvil.sortBubble x" \
  --allow-axiom "bv_decide" --allow-axiom "Lean.ofReduceBool"
$RAZOR submit --id SUB-ANV3 --hole ANV-003-NET-PROOF --solver judy --decl Razor.Anvil.network_refines
$RAZOR verify --submission SUB-ANV3
$RAZOR anvil-submit --id ANV-003-net --challenge ANV-003 --impl sort8-network \
  --solver judy --proof-decl Razor.Anvil.network_refines --refinement-hole ANV-003-NET-PROOF
$RAZOR bench --challenge ANV-003 --iters 20000 --rig wasm-referee
$RAZOR bench --challenge ANV-003 --iters 20000 --rig m4-station

step "ANV-100 EVM interpreter: the submission is admitted - and loses anyway"
$RAZOR challenge --id ANV-100 --title "EVM interpreter (64-bit demo words)" --spec-impl evm-ref \
  --obligation "∀ p gas stack, model p gas stack = Razor.Evm.execSpec p gas stack"
$RAZOR fund --target ANV-100 --amount 6000 --funder rollup-collective
$RAZOR anvil-submit --id ANV-100-ref --challenge ANV-100 --impl evm-ref \
  --solver spec-author --proof-decl ""
$RAZOR hole --id ANV-100-TOS-PROOF \
  --title "register-cached EVM interpreter refines the spec" \
  --lean-type "∀ (p : List Razor.Evm.Op) (gas : UInt64) (tos : Option UInt64) (rest : List UInt64), (tos = none → rest = []) → Razor.Evm.execTos p gas tos rest = Razor.Evm.execSpec p gas (Razor.Evm.stackOf tos rest)"
$RAZOR submit --id SUB-ANV100 --hole ANV-100-TOS-PROOF --solver leo --decl Razor.Evm.tos_refines
$RAZOR verify --submission SUB-ANV100
$RAZOR anvil-submit --id ANV-100-tos --challenge ANV-100 --impl evm-tos \
  --solver leo --proof-decl Razor.Evm.tos_refines --refinement-hole ANV-100-TOS-PROOF
$RAZOR bench --challenge ANV-100 --iters 5000 --rig wasm-referee
$RAZOR bench --challenge ANV-100 --iters 5000 --rig m4-station

step "Crown payouts stream to champions"
$RAZOR payout --target ANV-001 --recipient judy --amount 5000 \
  --reason "wasm-fuel and native crowns, popcount(u64)"
$RAZOR payout --target ANV-001 --recipient judy --amount 3000 \
  --reason "arch-reserved pool: crown on aarch64-apple-m (rig m4-station)"
$RAZOR payout --target ANV-003 --recipient judy --amount 4000 \
  --reason "wasm-fuel and native crowns, sort the 8 bytes of a u64"
$RAZOR payout --target ANV-002 --recipient kevin --amount 5000 \
  --reason "wasm-fuel and native crowns, sum(1..n)"

# ─────────────────────────────────────────────────────────────────────
step "ACT IV - Private submissions: commit-reveal (front-running protection)"
# ─────────────────────────────────────────────────────────────────────

step "nina seals her private RZR-106 proof and commits only the hash"
NINA_SALT="nina-keeps-this-secret-4217"
NINA_COMMIT=$($RAZOR seal --file examples/private/nina-rzr106.lean --salt "$NINA_SALT")
echo "commitment: $NINA_COMMIT"
$RAZOR commit --id SUB-106 --hole RZR-106 --solver nina --commitment "$NINA_COMMIT"

step "later, nina reveals; the registry checks the hash, installs, builds, verifies"
$RAZOR reveal --submission SUB-106 --file examples/private/nina-rzr106.lean \
  --salt "$NINA_SALT" --decl Razor.Private.Nina.isort_idempotent
$RAZOR verify --submission SUB-106

step "oscar commits to RZR-104 and stays sealed - priority without exposure"
$RAZOR commit --id SUB-104s --hole RZR-104 --solver oscar \
  --commitment "$($RAZOR seal --file examples/private/nina-rzr106.lean --salt oscar-wip)"

# ─────────────────────────────────────────────────────────────────────
step "ACT V - Real zero-knowledge: Groth16 submissions (zkGolf-style)"
# ─────────────────────────────────────────────────────────────────────

step "The circuit's meaning is itself a solved hole: constraints ⇒ sorted permutation"
$RAZOR hole --id ZKS-001 \
  --title "sorting-network R1CS constraints imply sorted + permutation" \
  --lean-type "∀ {x0 x1 x2 x3 a0 a1 a2 a3 b0 b1 b2 b3 c1 c2 : Int}, Razor.Zk.Cmp x0 x1 a0 a1 → Razor.Zk.Cmp x2 x3 a2 a3 → Razor.Zk.Cmp a0 a2 b0 b2 → Razor.Zk.Cmp a1 a3 b1 b3 → Razor.Zk.Cmp b1 b2 c1 c2 → (b0 ≤ c1 ∧ c1 ≤ c2 ∧ c2 ≤ b3) ∧ ∀ v, Razor.Zk.countZ v [b0, c1, c2, b3] = Razor.Zk.countZ v [x0, x1, x2, x3]"
$RAZOR submit --id SUB-ZKS --hole ZKS-001 --solver alice --decl Razor.Zk.network_sound
$RAZOR verify --submission SUB-ZKS
$RAZOR propose --id PRP-200 --author alice \
  --title "Field-to-integer transfer for the zk circuit" \
  --body "The Lean model works over Int; the circuit over F_p with values range-checked < 2^8. Formalize that the field constraints imply the integer ones. Open."

step "ZKH-001: an ordinary hole whose statement names one specific commitment"
# peggy publishes a commitment to her secret list. The hole pins a real
# Lean statement about that exact commitment - Razor.Zk.SortedWitnessFor,
# the Lean model of the circuit's commitment (public input decoded
# little-endian to a natural number). Anyone could in principle solve it
# with an ordinary Lean proof; peggy will solve it without revealing
# anything.
# The trusted setup is deterministic (fixed seed), so running it here and
# again inside `razor zk-route` produces the identical proving/verifying keys.
./target/release/zk-prover setup --n 4 > /dev/null
PEGGY=$(./target/release/zk-prover prove --list 42,7,255,7)
PEGGY_PUB=$(echo "$PEGGY" | python3 -c "import json,sys; print(json.load(sys.stdin)['public'])")
PEGGY_PRF=$(echo "$PEGGY" | python3 -c "import json,sys; print(json.load(sys.stdin)['proof'])")
PEGGY_NAT=$(python3 -c "print(int.from_bytes(bytes.fromhex('$PEGGY_PUB'), 'little'))")
$RAZOR hole --id ZKH-001 \
  --title "sorted witness for a committed list (4 values, 8-bit)" \
  --lean-type "Razor.Zk.SortedWitnessFor $PEGGY_NAT"
$RAZOR fund --target ZKH-001 --amount 4000 --funder privacy-dao

step "a zk route: an attachment that makes the hole solvable by a Groth16 proof"
# The route pins the verifying key and the bridge: a kernel-checked Lean
# theorem that the circuit's constraints imply the pinned statement. The
# same mechanism could carry a universal route (a proof checker run inside
# a zkVM), whose bridge is a binary hash instead of a theorem.
$RAZOR zk-route --id ZKR-001 --hole ZKH-001 \
  --bridge-kind theorem --bridge Razor.Zk.network_sound \
  --note "bridge covers the sorting network; the commitment binding and the field-to-integer transfer are the open gaps PRP-200 records"

step "peggy proves knowledge of her secret list (the registry never sees it)"
$RAZOR zk-submit --id SUB-ZK1 --hole ZKH-001 --route ZKR-001 --solver peggy \
  --public "$PEGGY_PUB" --proof "$PEGGY_PRF"
$RAZOR zk-verify --submission SUB-ZK1

step "a forged proof bounces off the verifier"
$RAZOR zk-submit --id SUB-ZK2 --hole ZKH-001 --route ZKR-001 --solver mallory \
  --public "$PEGGY_PUB" --proof "${PEGGY_PRF%????}0000"
$RAZOR zk-verify --submission SUB-ZK2 || true

# ─────────────────────────────────────────────────────────────────────
step "Final registry state"
# ─────────────────────────────────────────────────────────────────────
$RAZOR status

step "Export for the site"
$RAZOR export --out site/data.json --dataset demo
echo
echo "Done. Serve the site with:  target/release/razor serve   (live: pages update as events land)"
