// Shared loader + helpers for the Satoshi's Razor site.
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));

const NAV = [
  ['index.html', 'overview'],
  ['frontier.html', 'frontier'],
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
  sealed: 'This statement was filed as a hash commitment before it was shown to anyone; the reveal matched the hash. It is provably blind to every statement revealed after its seal.',
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
  const attention = pool + 900 * clump + 700 * curation + 800 * convergence + 600 * lineage
    + 500 * splits + 250 * subs + (h.status === 'open' ? 400 : 0) - 700 * superseded;
  return { clump, dominant: !!clumpOf?.dominant,
    proven: !!clumpOf?.proven, pool, curation, convergence, lineage, splits, subs, rejected,
    superseded, attention };
}

// every event touching a hole, in log order
function holeHistory(S, h) {
  const subIds = new Set(h.submissions.map(s => s.id));
  return S.events.filter(e => {
    switch (e.type) {
      case 'register_hole': return e.id === h.id;
      case 'fund': case 'payout': case 'curate': return e.target === h.id;
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
