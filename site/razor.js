// Shared loader + helpers for the Satoshi's Razor site.
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));

const NAV = [
  ['index.html', 'overview'],
  ['frontier.html', 'frontier'],
  ['how.html', 'how it works'],
  ['anvil.html', 'the anvil'],
  ['zk.html', 'zero-knowledge'],
  ['people.html', 'people'],
  ['download.html', 'get started'],
];

const REPO = 'https://github.com/jorikschellekens/satoshis-razor';

function renderNav() {
  const here = location.pathname.split('/').pop() || 'index.html';
  const el = document.querySelector('nav.pages');
  if (el) el.innerHTML = NAV.map(([href, label]) =>
    `<a href="${href}" class="${href === here ? 'here' : ''}">${label}</a>`).join('');
  // Every page links the source: the site is computed from the repository's
  // log, and participating means sending a pull request to it.
  const right = document.querySelector('footer .right');
  if (right && !right.querySelector('a[href^="https://github"]'))
    right.insertAdjacentHTML('beforeend', ` · <a href="${REPO}">source on GitHub</a>`);
}

function loadData(render) {
  renderNav();
  let eventCount = -1;
  const apply = (S) => {
    if (S.events.length === eventCount) return;
    eventCount = S.events.length;
    renderDatasetBanner(S);
    render(S);
  };
  fetch('data.json').then(r => r.json()).then(S => {
    apply(S);
    // Live updates: when served by `razor serve`, data.json is re-derived
    // from the log on every request, so polling picks up new events.
    if (location.protocol.startsWith('http')) {
      setInterval(() => fetch('data.json').then(r => r.json()).then(apply).catch(() => {}), 4000);
    }
  }).catch(() => {
    const t = document.querySelector('.pagehead .lede') || document.body;
    t.textContent = 'No data.json found. Run ./seed.sh (live registry) or ./demo.sh (scripted walkthrough) at the repo root, then reload.';
  });
}

// ── entity links ─────────────────────────────────────────────────
// Every id on the site is a link to that entity's own page.
const qid = () => new URLSearchParams(location.search).get('id');
const holeLink = (id) => `<a class="idlink" href="hole.html?id=${encodeURIComponent(id)}">${esc(id)}</a>`;
const stmtLink = (id) => `<a class="idlink" href="statement.html?id=${encodeURIComponent(id)}">${esc(id)}</a>`;
const propLink = (id) => `<a class="idlink" href="proposal.html?id=${encodeURIComponent(id)}">${esc(id)}</a>`;
const personLink = (h) => `<a class="idlink" href="person.html?id=${encodeURIComponent(h)}">${esc(h)}</a>`;
const chalLink = (id) => `<a class="idlink" href="challenge.html?id=${encodeURIComponent(id)}">${esc(id)}</a>`;

// Every export is labeled with the dataset it came from. The demo dataset
// is a scripted walkthrough with fictional participants (the verifications
// and benchmarks in it are real); the live dataset is the actual registry.
function renderDatasetBanner(S) {
  if (S.dataset !== 'demo') return;
  const mast = document.querySelector('header.mast');
  if (!mast) return;
  const b = document.createElement('div');
  b.className = 'dataset-banner';
  b.innerHTML = '<div class="wrap">You are viewing the <strong>demo dataset</strong>: a scripted' +
    ' walkthrough with fictional participants, used to exercise every mechanism. The proof checks' +
    ' and benchmark numbers in it are real. Run <code>./seed.sh</code> for the live registry.</div>';
  mast.after(b);
}

const CHIP = { open: ['○', 'open'], solved: ['✓', 'solved'] };
const chip = (status) => {
  const [g, l] = CHIP[status] || ['·', status];
  return `<span class="chip ${esc(status)}">${g} ${l}</span>`;
};

// Typography for prose and titles ONLY - never for Lean code, where the
// pinned statement is exact character for character.
const prettyMath = (s) => String(s)
  .replace(/ >= /g, ' ≥ ').replace(/ <= /g, ' ≤ ')
  .replace(/ != /g, ' ≠ ').replace(/ <-> /g, ' ↔ ').replace(/ -> /g, ' → ');

// Minimal Lean syntax highlighting: comments, strings, sorry, keywords.
// Takes raw source, returns escaped HTML.
function hiLean(src) {
  const re = /(\/--[\s\S]*?-\/|\/-[\s\S]*?-\/|--[^\n]*)|("(?:[^"\\]|\\.)*")|\b(sorry)\b|\b(theorem|lemma|example|def|abbrev|structure|inductive|instance|class|where|deriving|import|namespace|end|open|section|universe|variable|by|fun|let|have|show|from|match|with|do|then|else|if|calc|Prop|Type|Sort)\b/g;
  let out = '', last = 0;
  for (let m; (m = re.exec(src)); ) {
    out += esc(src.slice(last, m.index));
    const cls = m[1] ? 'lc' : m[2] ? 'lstr' : m[3] ? 'lsorry' : 'lk';
    out += `<span class="${cls}">${esc(m[0])}</span>`;
    last = m.index + m[0].length;
  }
  return out + esc(src.slice(last));
}

// Highlighted Lean with each Mathlib-resolved identifier linked to the
// Mathlib documentation.
const MATHLIB_DOC = (n) => `https://leanprover-community.github.io/mathlib4_docs/find/?pattern=${encodeURIComponent(n)}#doc`;
function hiLeanLinked(src, mathlibNames) {
  let html = hiLean(src);
  for (const n of (mathlibNames || [])) {
    const re = new RegExp(`\\b${esc(n).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`, 'g');
    html = html.replace(re, `<a class="mlink" href="${MATHLIB_DOC(n)}" title="open ${esc(n)} in the Mathlib documentation">${esc(n)}</a>`);
  }
  return html;
}

// The informal reading of a hole: the statement author's gloss if there is
// one, else the proposal's plain-language body.
function informalOf(S, h) {
  const st = h.statement && S.statements?.[h.statement];
  const p = h.proposal && S.proposals?.[h.proposal];
  return (st && st.gloss) || (p && p.body) || '';
}

// One-line definitions, attached to badges as hover text so the site's
// vocabulary travels with the reader.
const TIP = {
  window: 'A dated invitation to file sealed readings of this proposal. Nothing is enforced: a late seal simply carries its own timestamps.',
  sealed: 'This statement was filed as a hash commitment before any other reading of the proposal was public; the reveal matched the hash. It is provably blind to every statement revealed after its seal.',
  sealedLate: 'This statement was filed as a hash commitment, but other readings of the proposal were already public when it was sealed - the seal timestamps it without proving blindness against those.',
  tag: 'An attributed label filed with razor tag. test-data marks pipeline-test material: it stays recorded and provable, but default views de-emphasize it and the homepage marquee leaves it out.',
  blind: 'The largest set of authors in this clump whose statements were each sealed before any of the others was revealed: none could have seen another’s Lean. Weight counts claimed independence; this counts proof.',
  bridge: 'A hole whose pinned statement is the equivalence of two candidate statements. An admitted proof is a kernel-checked fact that they state the same problem, and their clumps merge.',
  fidelity: 'Recorded facts about how much independent scrutiny the pinned statement has survived. The registry never judges a statement; these are what the log knows.',
  canonical: "The pinned type is Mathlib's own statement of the theorem - not a local translation of it.",
  clump: 'Statements proven equivalent by machine-checked proof form a clump; its weight counts distinct authors.',
  dominant: 'The unique heaviest clump with at least two independent authors - the reading the community has converged on.',
  proven: 'Some member of the clump has an admitted proof; proving one member proves them all.',
  curation: "Public, attributed picks of problems worth working on, weighted by the curator's admitted work.",
  convergence: "Machine-checked equivalence proofs on this hole's statement.",
  lineage: 'Earlier wordings whose supersession marks point here: how many times this problem has been re-stated.',
  superseded: "An attributed note that a better wording exists. It closes nothing; it is weighted by the filer's admitted work.",
  splits: 'Registered plans reducing this hole to child holes, each with a machine-checked glue proof that the children suffice.',
  submissions: 'Claimed solutions, each checked by the Lean kernel.',
  attention: 'Bounty credits plus fixed weights for each community signal, minus supersession marks. A reading aid, not a judgment.',
  bounty: 'Credits attached to this exact statement. The first admitted proof is paid, with no adjudication.',
  env: "Stated using Mathlib's definitions and checked in the Mathlib environment.",
  upstreamed: 'The admitted proof was carried to its home library; the pull request is recorded on the log.',
};
const tip = (k) => TIP[k] ? ` title="${esc(TIP[k])}"` : '';

// Shown on a detail page when an id resolves to nothing: the most common
// cause is a link that lives in the other dataset.
const datasetHint = (S) => S.dataset === 'demo'
  ? 'This site is currently showing the <b>demo dataset</b>. If you followed a link to a live-registry entity, run <code>./seed.sh</code> and reload.'
  : 'This site is currently showing the <b>live registry</b>. If you followed a link from the demo walkthrough, run <code>./demo.sh</code> and reload.';

// ── challenge windows and sealed readings ────────────────────────
const fmtDay = (ts) => new Date(ts * 1000).toLocaleDateString(undefined,
  { year: 'numeric', month: 'short', day: 'numeric' });

// The proposal's currently relevant window, if any: the latest one whose
// reveal deadline has not passed ("sealing" or "revealing"), else null.
function activeRound(S, p) {
  const now = Date.now() / 1000;
  const rounds = (p.rounds || []).map(id => S.rounds?.[id]).filter(Boolean);
  const live = rounds.filter(r => now < r.reveal_by);
  if (!live.length) return null;
  const r = live[live.length - 1];
  return { ...r, phase: now < r.closes_at ? 'sealing' : 'revealing' };
}

// Seals on a proposal that have not been revealed yet.
const pendingSeals = (S, p) => (p.seals || [])
  .map(id => S.seals?.[id]).filter(s => s && !s.statement);

// Two statements are mutually blind when each was committed (sealed - or,
// unsealed, filed) before the other was revealed: neither author could
// have seen the other's Lean. This is the pairwise fact behind a clump's
// "written blind" count.
function mutuallyBlind(x, y) {
  const c = (s) => s.sealed_seq ?? s.filed_seq;
  return c(x) < y.filed_seq && c(y) < x.filed_seq;
}
function blindPeersOf(S, st) {
  const p = S.proposals?.[st.proposal];
  return (p?.statements || [])
    .filter(id => id !== st.id)
    .map(id => S.statements[id])
    .filter(o => o && o.author !== st.author && mutuallyBlind(st, o));
}

// ── tags ─────────────────────────────────────────────────────────
// Attributed labels on any entity (razor tag). The one tag the site itself
// acts on is test-data: tagged items keep their pages, stay provable, and
// remain on the log - but default views de-emphasize them, and the
// homepage marquee leaves them out.
function tagsOf(S, id) {
  return (S.tags || []).filter(([, t]) => t === id).map(([by, , tag, note]) => ({ by, tag, note }));
}
const isTestData = (S, id) => tagsOf(S, id).some(t => t.tag === 'test-data');
// A hole inherits test-data from its proposal: tagging the audit proposal
// covers every hole filed under it.
const holeIsTest = (S, h) => isTestData(S, h.id) || (h.proposal && isTestData(S, h.proposal));
function tagChips(S, id) {
  return tagsOf(S, id).map(t =>
    `<span class="badge"${tip('tag')}>⚑ ${esc(t.tag)} — tagged by ${esc(t.by)}${t.note ? `: “${esc(t.note)}”` : ''}</span>`).join('');
}

// How blind a sealed statement provably was: the number of same-proposal
// statements by other authors that were already public when this one was
// sealed. 0 means the seal predates every other reading - the badge can
// honestly say "blind"; more means the author could have read them first.
function sealLateness(S, st) {
  if (st.sealed_seq == null) return null;
  const p = S.proposals?.[st.proposal];
  return (p?.statements || [])
    .map(id => S.statements[id])
    .filter(o => o && o.id !== st.id && o.author !== st.author)
    .filter(o => o.filed_seq < st.sealed_seq).length;
}
// The sealed badge, worded honestly: a seal that predates every other
// public reading is provably blind; a later seal only timestamps itself.
function sealedBadge(S, st) {
  if (st.sealed_seq == null) return '';
  const late = sealLateness(S, st);
  if (late === 0) return `<span class="badge gold"${tip('sealed')}>⏣ sealed blind at #${st.sealed_seq}</span>`;
  return `<span class="badge"${tip('sealedLate')}>⏣ sealed at #${st.sealed_seq} · ${late} reading${late > 1 ? 's' : ''} already public</span>`;
}

// Curation weight of a target: each curation counts 1 plus the curator's
// admitted work, so taste from people with a verified record counts more.
function curationWeight(S, target) {
  return (S.curations || []).filter(([, t]) => t === target)
    .reduce((a, [who]) => a + 1 + (S.people?.[who]?.solved || 0), 0);
}
function curatorsOf(S, target) {
  return (S.curations || []).filter(([, t]) => t === target);
}

// Weight of the supersession marks on a hole, computed exactly like
// curation weight: each mark counts 1 plus the filer's admitted work.
function supersedeWeight(S, hole) {
  return (S.supersessions || []).filter(([, h]) => h === hole)
    .reduce((a, [who]) => a + 1 + (S.people?.[who]?.solved || 0), 0);
}

// ── derived metrics for the frontier ─────────────────────────────
// clump: weight (distinct independent authors) of the equivalence clump the
// hole's statement belongs to; lineage: length of the supersession chain
// through this hole; convergence: equivalence proofs on its statement.
function holeMetrics(S, h) {
  const stmt = h.statement ? S.statements[h.statement] : null;
  const prop = h.proposal ? S.proposals[h.proposal] : null;
  const clumpOf = prop && h.statement
    ? (prop.clumps || []).find(c => c.members.includes(h.statement)) : null;
  const clump = clumpOf ? clumpOf.weight : 0;
  const pool = h.pool;
  const curation = curationWeight(S, h.id);
  const convergence = stmt ? stmt.convergences.length : 0;
  let lineage = 0;
  // walk backward: holes carrying a supersession mark that points at
  // (a chain ending at) this hole
  const preds = (id) => Object.values(S.holes)
    .filter(x => (x.superseded_by || []).some(([, r]) => r === id));
  let frontier = [h.id];
  while (frontier.length) {
    const prev = frontier.flatMap(id => preds(id).map(x => x.id));
    lineage += prev.length;
    frontier = prev;
  }
  const splits = (h.splits || []).length;
  const subs = h.submissions.length + (h.zk_submissions || []).length;
  const rejected = h.submissions.filter(s => s.verdict && !s.verdict[0]).length;
  const superseded = supersedeWeight(S, h.id);
  // attention: a single sortable number estimating how much this hole
  // matters to the community right now. Bounty credits count at face value;
  // each community signal counts at a fixed weight; supersession marks
  // subtract. It is a reading aid, not a judgment - every input is visible
  // on the hole's own page.
  const test = holeIsTest(S, h);
  const attention = pool + 900 * clump + 700 * curation + 800 * convergence + 600 * lineage
    + 500 * splits + 250 * subs + (h.status === 'open' ? 400 : 0) - 700 * superseded
    - (test ? 5000 : 0);
  return { clump, dominant: !!clumpOf?.dominant,
    proven: !!clumpOf?.proven, pool, curation, convergence, lineage, splits, subs, rejected,
    superseded, test, attention };
}

// every event touching a hole, in log order
function holeHistory(S, h) {
  const subIds = new Set(h.submissions.map(s => s.id));
  return S.events.filter(e => {
    switch (e.type) {
      case 'register_hole': return e.id === h.id;
      case 'fund': case 'payout': case 'curate': case 'tag': return e.target === h.id;
      case 'submit': case 'commit': return e.hole === h.id;
      case 'reveal': case 'verdict': return subIds.has(e.submission);
      case 'supersede': return e.hole === h.id || e.replacement === h.id;
      case 'repin': case 'upstream': return e.hole === h.id;
      case 'zk_route': case 'zk_submit': return e.hole === h.id;
      case 'split': return e.parent === h.id || (e.children || []).includes(h.id) || e.glue === h.id;
      case 'formalize': return h.statement && e.id === h.statement;
      case 'seal_statement': return h.statement && S.statements[h.statement]?.seal === e.id;
      case 'reveal_statement': return h.statement && e.statement === h.statement;
      case 'certify': return h.statement && e.statement === h.statement;
      case 'converge': case 'implies': return h.statement && (e.a === h.statement || e.b === h.statement);
      default: return false;
    }
  });
}

const EV_TONE = {
  verdict: e => e.admitted ? 'good' : 'bad',
  supersede: () => 'bad',
  payout: () => 'gold', fund: () => 'gold', curate: () => 'gold',
  upstream: () => 'gold',
};

function evLine(e) {
  const { seq, ts, type, ...rest } = e;
  // The 128-hex signature would drown the row; verify-log checks it, the
  // row just notes it is there.
  let detail = Object.entries(rest)
    .filter(([k, v]) => v !== '' && v != null && !(Array.isArray(v) && !v.length)
      && !['lean_type', 'body', 'detail', 'notes', 'obligation', 'proof', 'public', 'vk_path', 'commitment', 'statement', 'sig'].includes(k))
    .map(([k, v]) => `${k}=${Array.isArray(v) ? v.join('|') : v}`).join('  ');
  if (rest.sig) detail += '  · signed';
  return { seq, type, detail, tone: (EV_TONE[type] || (() => ''))(e) };
}

function timeline(events) {
  return `<div class="timeline">` + events.map(e => {
    const { seq, type, detail, tone } = evLine(e);
    return `<div class="tl ${tone}"><span class="k">#${seq} ${esc(type)}</span>  ${esc(detail)}</div>`;
  }).join('') + `</div>`;
}

// ── test-data closure over the log ───────────────────────────────
// Everything reachable from a test-data tag: the tagged entities, the
// holes under tagged proposals, and the submissions/seals/statements those
// entities and handles produced. Used to keep pipeline-test noise out of
// default feeds without touching the log itself.
function testIdSet(S) {
  if (S._testIds) return S._testIds;
  const set = new Set((S.tags || []).filter(([, , t]) => t === 'test-data').map(([, id]) => id));
  for (const p of Object.values(S.proposals || {}))
    if (set.has(p.id)) for (const sid of (p.statements || [])) set.add(sid);
  for (const h of Object.values(S.holes || {}))
    if (set.has(h.id) || (h.proposal && set.has(h.proposal))) {
      set.add(h.id);
      for (const s of (h.submissions || [])) set.add(s.id);
      for (const sp of (h.splits || [])) set.add(sp.id);
    }
  for (const h of Object.values(S.holes || {}))
    for (const s of (h.submissions || [])) if (set.has(s.solver)) set.add(s.id);
  for (const sl of Object.values(S.seals || {}))
    if (set.has(sl.author) || (sl.statement && set.has(sl.statement))) set.add(sl.id);
  for (const st of Object.values(S.statements || {}))
    if (set.has(st.id) && st.seal) set.add(st.seal);
  return (S._testIds = set);
}
// An event is test noise if any id it mentions is in the closure.
function eventIsTest(S, e) {
  const set = testIdSet(S);
  const scan = (v) => typeof v === 'string' ? set.has(v)
    : Array.isArray(v) ? v.some(scan) : false;
  return Object.entries(e).some(([k, v]) => k !== 'type' && scan(v));
}

// ── the activity feed: log events as sentences ───────────────────
function timeAgo(ts) {
  const s = Date.now() / 1000 - ts;
  if (s < 90) return 'just now';
  const m = s / 60, h = m / 60, d = h / 24;
  if (m < 90) return `${Math.round(m)} min ago`;
  if (h < 36) return `${Math.round(h)} hours ago`;
  if (d < 60) return `${Math.round(d)} days ago`;
  return fmtDay(ts);
}
const daysLeft = (ts) => {
  const d = Math.ceil((ts - Date.now() / 1000) / 86400);
  return d <= 0 ? 'closes today' : d === 1 ? '1 day left' : `${d} days left`;
};

// Each event as one plain-language sentence with its entities linked.
// Returns null for types better left to the raw ledger.
function humanEvent(S, e) {
  const holeOfSub = (id) => Object.values(S.holes || {})
    .find(h => (h.submissions || []).some(s => s.id === id)
      || (h.zk_submissions || []).some(s => s.id === id));
  const q = (s) => s ? ` — “${esc(String(s).length > 90 ? String(s).slice(0, 90) + '…' : s)}”` : '';
  switch (e.type) {
    case 'propose': return { t: '', h: `${personLink(e.author)} proposed ${propLink(e.id)}${q(e.title)}` };
    case 'formalize': return { t: '', h: `${personLink(e.author)} formalized ${propLink(e.proposal)} as statement ${stmtLink(e.id)}` };
    case 'seal_statement': return { t: 'gold', h: `${personLink(e.author)} sealed a blind reading of ${propLink(e.proposal)}` };
    case 'reveal_statement': return { t: '', h: `${personLink(e.author)} revealed sealed statement ${stmtLink(e.statement)}` };
    case 'open_round': return { t: 'gold', h: `${personLink(e.author)} opened a challenge window on ${propLink(e.proposal)}` };
    case 'register_hole': return { t: '', h: `hole ${holeLink(e.id)} pinned${e.author ? ` by ${personLink(e.author)}` : ''}${q(e.title)}` };
    case 'submit': return { t: '', h: `${personLink(e.solver)} submitted a proof against ${holeLink(e.hole)}` };
    case 'verdict': {
      const h = holeOfSub(e.submission);
      const where = h ? ` on ${holeLink(h.id)}` : '';
      return e.admitted
        ? { t: 'good', h: `the kernel admitted ${esc(e.submission)}${where}${e.cost_ms != null ? ` (checked in ${e.cost_ms} ms)` : ''}` }
        : { t: 'bad', h: `the kernel rejected ${esc(e.submission)}${where}` };
    }
    case 'fund': return { t: 'gold', h: `${personLink(e.funder)} put ${(+e.amount).toLocaleString()} credits on ${holeLink(e.target)}` };
    case 'payout': return { t: 'gold', h: `${personLink(e.recipient)} was paid ${(+e.amount).toLocaleString()} credits for ${holeLink(e.target)}` };
    case 'curate': return { t: 'gold', h: `${personLink(e.curator)} curated ${esc(e.target)}${q(e.note)}` };
    case 'supersede': return { t: 'bad', h: `${personLink(e.by)} marked ${holeLink(e.hole)} superseded by ${holeLink(e.replacement)}` };
    case 'split': return { t: '', h: `${personLink(e.author)} split ${holeLink(e.parent)} into ${(e.children || []).length} subproblem${(e.children || []).length === 1 ? '' : 's'}` };
    case 'converge': return { t: 'good', h: `statements ${stmtLink(e.a)} and ${stmtLink(e.b)} proven equivalent — their clumps merge` };
    case 'implies': return { t: '', h: `${stmtLink(e.a)} proven to imply ${stmtLink(e.b)}` };
    case 'certify': return { t: '', h: `sanity certificate recorded on ${stmtLink(e.statement)}` };
    case 'register_account': return { t: '', h: `${esc(e.sigil || '')} ${personLink(e.handle)} registered a handle` };
    case 'recognize_corpus': return { t: '', h: `corpus <b>${esc(e.name)}</b> recognized — its contents count as already solved` };
    case 'commit': return { t: '', h: `${personLink(e.solver)} committed a sealed solution to ${holeLink(e.hole)}` };
    case 'reveal': return { t: '', h: `submission ${esc(e.submission)} revealed — the file matches its committed hash` };
    case 'upstream': return { t: 'gold', h: `an admitted proof of ${holeLink(e.hole)} was carried to its home library` };
    case 'tag': return { t: '', h: `${personLink(e.by)} tagged ${esc(e.target)} <code>${esc(e.tag)}</code>` };
    default: return null;
  }
}

function renderFeedInto(id, S, limit = 10) {
  const el = $(id);
  if (!el) return;
  const rows = [];
  for (let i = S.events.length - 1; i >= 0 && rows.length < limit; i--) {
    const e = S.events[i];
    if (eventIsTest(S, e)) continue;
    const s = humanEvent(S, e);
    if (s) rows.push(`<div class="feedrow ${s.t}"><span class="ft">${timeAgo(e.ts)}</span><span class="fs">${s.h}</span><span class="fseq">#${e.seq}</span></div>`);
  }
  el.innerHTML = rows.join('') || '<p class="lede">Nothing yet.</p>';
}

// ── progress bars ────────────────────────────────────────────────
// Solved-vs-open as segments when countable at a glance, a fill otherwise.
function pbar(done, total, word = 'solved') {
  if (!total) return '';
  const bar = total <= 24
    ? `<span class="pbar">${Array.from({ length: total }, (_, i) => `<i class="${i < done ? 'done' : ''}"></i>`).join('')}</span>`
    : `<span class="pbar cont"><i class="done" style="width:${Math.round(100 * done / total)}%"></i></span>`;
  return `<span class="progress" role="img" aria-label="${done} of ${total} ${word}">${bar}<span class="plabel">${done} of ${total} ${word}</span></span>`;
}

// ── first solves ─────────────────────────────────────────────────
// Who holds priority: for each solved hole, the solver of the first
// admitted submission. The board the whole system is built around.
function firstSolves(S) {
  const per = {};
  for (const h of Object.values(S.holes || {})) {
    if (h.status !== 'solved' || holeIsTest(S, h)) continue;
    const sub = (h.submissions || []).find(s => s.id === h.solved_by)
      || (h.submissions || []).find(s => s.verdict && s.verdict[0]);
    if (!sub) continue;
    (per[sub.solver] ||= []).push(h.id);
  }
  return Object.entries(per).map(([who, holes]) => ({ who, holes, n: holes.length }))
    .sort((a, b) => b.n - a.n || a.who.localeCompare(b.who));
}

// ── the hole graph, shared by the frontier and the how-it-works page ──
// Layered by longest edge distance; supersession and split edges.
function renderHoleGraph(svgId, S) {
  const holes = Object.values(S.holes || {});
  const byId = Object.fromEntries(holes.map(h => [h.id, h]));
  const edges = [];
  const seen = new Set();
  const addEdge = (from, to, kind) => {
    const key = `${from}|${to}|${kind}`;
    if (byId[to] && !seen.has(key)) { seen.add(key); edges.push({ from, to, kind }); }
  };
  for (const h of holes) {
    if (holeIsTest(S, h)) continue;
    for (const [, r] of (h.superseded_by || [])) addEdge(h.id, r, 'supersede');
    for (const sp of (h.splits || []))
      for (const c of sp.children) addEdge(h.id, c[0], 'decompose');
  }
  const inGraph = new Set(edges.flatMap(e => [e.from, e.to]));
  const el = $(svgId);
  if (!inGraph.size) { el.closest('section').style.display = 'none'; return; }
  const depth = {};
  const dfs = (id) => {
    if (id in depth) return depth[id];
    const preds = edges.filter(e => e.to === id);
    depth[id] = preds.length ? 1 + Math.max(...preds.map(e => dfs(e.from))) : 0;
    return depth[id];
  };
  inGraph.forEach(dfs);
  const cols = {};
  inGraph.forEach(id => (cols[depth[id]] ||= []).push(id));
  const tallest = Math.max(...Object.values(cols).map(ids => ids.length));
  const W = 940, colW = W / Object.keys(cols).length, H = Math.max(250, tallest * 64);
  const pos = {};
  Object.entries(cols).forEach(([d, ids]) => {
    ids.sort();
    ids.forEach((id, i) => { pos[id] = { x: colW * d + colW / 2, y: (H / (ids.length + 1)) * (i + 1) + 14 }; });
  });
  const color = { open: 'var(--open)', solved: 'var(--solved)' };
  let svg = '';
  for (const e of edges) {
    const a = pos[e.from], b = pos[e.to];
    const mx = (a.x + b.x) / 2, wob = (e.to.charCodeAt(e.to.length - 1) % 2 ? 14 : -12);
    svg += `<path class="gedge ${e.kind}" d="M ${a.x + 62} ${a.y} Q ${mx} ${(a.y + b.y) / 2 + wob} ${b.x - 62} ${b.y}"/>`;
    if (e.kind === 'supersede')
      svg += `<text class="gedge-label" x="${mx}" y="${(a.y + b.y) / 2 + wob - 6}" text-anchor="middle">marked superseded by</text>`;
  }
  for (const id of inGraph) {
    const h = byId[id], p = pos[id];
    const [glyph] = CHIP[h.status];
    svg += `<a href="hole.html?id=${encodeURIComponent(id)}"><g class="gnode">
      <title>${esc(prettyMath(h.title))}</title>
      <ellipse cx="${p.x}" cy="${p.y}" rx="64" ry="26" fill="var(--bg)" stroke="${color[h.status]}" stroke-width="1.8"/>
      <text x="${p.x}" y="${p.y - 1}" text-anchor="middle">${esc(id)}</text>
      <text class="sub" x="${p.x}" y="${p.y + 13}" text-anchor="middle">${glyph} ${h.status}</text>
    </g></a>`;
  }
  el.setAttribute('viewBox', `0 0 ${W} ${H + 20}`);
  el.innerHTML = svg;
}

function renderLedgerInto(id, events, limit = 100) {
  const el = $(id);
  if (!el) return;
  const row = e => {
    const { seq, type, detail } = evLine(e);
    let k = type;
    if (type === 'verdict') k += `-${e.admitted}`;
    return `<div class="l"><span class="seq">${seq}</span><span class="k ${esc(type)} ${esc(k)}">${esc(type)}</span><span>${esc(detail)}</span></div>`;
  };
  // The whole log on one page is thousands of lines; show the recent tail
  // and expand on demand. The full file is always one click away anyway
  // (registry/data/events.jsonl in the repository).
  const all = el.dataset.expanded === '1' || events.length <= limit;
  const shown = all ? events : events.slice(-limit);
  const expander = all ? '' :
    `<div class="l"><span class="seq">…</span><span class="k"></span><span><a href="#" id="${id}-more">${(events.length - shown.length).toLocaleString()} earlier events collapsed — show the whole log</a></span></div>`;
  el.innerHTML = expander + shown.map(row).join('');
  const more = $(id + '-more');
  if (more) more.onclick = (ev) => {
    ev.preventDefault();
    el.dataset.expanded = '1';
    renderLedgerInto(id, events, limit);
  };
}
