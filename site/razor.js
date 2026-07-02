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

function renderNav() {
  const here = location.pathname.split('/').pop() || 'index.html';
  const el = document.querySelector('nav.pages');
  if (el) el.innerHTML = NAV.map(([href, label]) =>
    `<a href="${href}" class="${href === here ? 'here' : ''}">${label}</a>`).join('');
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
      case 'zk_route': case 'zk_submit': return e.hole === h.id;
      case 'split': return e.parent === h.id || (e.children || []).includes(h.id) || e.glue === h.id;
      case 'formalize': return h.statement && e.id === h.statement;
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
};

function evLine(e) {
  const { seq, ts, type, ...rest } = e;
  const detail = Object.entries(rest)
    .filter(([k, v]) => v !== '' && v != null && !(Array.isArray(v) && !v.length)
      && !['lean_type', 'body', 'detail', 'notes', 'obligation', 'proof', 'public', 'vk_path', 'commitment', 'statement'].includes(k))
    .map(([k, v]) => `${k}=${Array.isArray(v) ? v.join('|') : v}`).join('  ');
  return { seq, type, detail, tone: (EV_TONE[type] || (() => ''))(e) };
}

function timeline(events) {
  return `<div class="timeline">` + events.map(e => {
    const { seq, type, detail, tone } = evLine(e);
    return `<div class="tl ${tone}"><span class="k">#${seq} ${esc(type)}</span>  ${esc(detail)}</div>`;
  }).join('') + `</div>`;
}

function renderLedgerInto(id, events) {
  const el = $(id);
  if (!el) return;
  el.innerHTML = events.map(e => {
    const { seq, type, detail } = evLine(e);
    let k = type;
    if (type === 'verdict') k += `-${e.admitted}`;
    return `<div class="l"><span class="seq">${seq}</span><span class="k ${esc(type)} ${esc(k)}">${esc(type)}</span><span>${esc(detail)}</span></div>`;
  }).join('');
}
