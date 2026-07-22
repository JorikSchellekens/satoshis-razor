// Shared code for the anvil pages: lane colors, board grouping, champions,
// and the score-over-time charts. Loaded after razor.js.

// Styles owned by this module (the record line in the charts).
document.head.insertAdjacentHTML('beforeend', `<style>
.crecord { stroke: var(--chalk); opacity: 0.5; stroke-dasharray: 5 4; }
.crecord-pt { fill: var(--chalk); opacity: 0.7; }
.crecord-label { fill: var(--muted); }
</style>`);

// Lane colors: fixed assignment by the order lanes joined the challenge
// (the specification lane is first and always blue). Validated for this
// site's dark surface, including for color-blind readers.
const LANE_COLORS = ['#348AC9', '#B28D3B', '#9B7FD4', '#C85850'];
const laneColor = (i) => LANE_COLORS[i % LANE_COLORS.length];
const laneDot = (i) => `<span class="dot" style="background:${laneColor(i)}"></span>`;

const fmtScore = (v) => v >= 100 ? v.toLocaleString(undefined, { maximumFractionDigits: 0 })
  : v.toLocaleString(undefined, { maximumFractionDigits: 2 });
const fmtWhen = (ts) => new Date(ts * 1000).toLocaleDateString(undefined,
  { month: 'short', day: 'numeric' }) + ' ' +
  new Date(ts * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
const fmtDayShort = (ts) => new Date(ts * 1000).toLocaleDateString(undefined,
  { month: 'short', day: 'numeric' });

// ── the data model of a board ────────────────────────────────────
// A board is one tier measured on one rig: scores on it are directly
// comparable. Board keys are "tier|arch|rig".
const boardKey = (s) => `${s.tier}|${s.arch}|${s.rig || ''}`;
const boardParts = (key) => { const [tier, arch, rig] = key.split('|'); return { tier, arch, rig }; };
const boardOrder = (a, b) =>
  (a.startsWith('wasm-fuel') ? 0 : 1) - (b.startsWith('wasm-fuel') ? 0 : 1) || a.localeCompare(b);

// Current standings per board of one challenge: { key: [{e, lane, ...score}] },
// each list sorted fastest first.
function boardsOf(c) {
  const boards = {};
  c.entries.forEach((e, lane) => {
    for (const s of e.scores) (boards[boardKey(s)] ||= []).push({ e, lane, ...s });
  });
  for (const rows of Object.values(boards)) rows.sort((a, b) => a.score - b.score);
  return boards;
}

// All bench events, resolved to (challenge, entry, lane index). The event
// log keeps every measurement; the derived state keeps only the latest per
// board - history comes from here.
function benchHistory(S) {
  const laneOf = {};
  for (const c of Object.values(S.challenges))
    c.entries.forEach((e, i) => { laneOf[e.id] = { c, e, i }; });
  return S.events.filter(ev => ev.type === 'bench' && laneOf[ev.submission])
    .map(ev => ({ ...ev, ...laneOf[ev.submission] }));
}

// Measurements of one challenge on one board, grouped into chart series
// (one per lane), oldest first.
function boardSeries(history, c, key) {
  const seriesMap = {};
  for (const b of history) {
    if (b.c.id !== c.id || boardKey(b) !== key) continue;
    (seriesMap[b.submission] ||= { name: b.e.impl_name, lane: b.i, points: [] })
      .points.push({ ts: b.ts, score: b.score });
  }
  for (const s of Object.values(seriesMap)) s.points.sort((a, b) => a.ts - b.ts);
  return Object.values(seriesMap);
}

// The record line of a board: the fastest result anyone had recorded up
// to each moment in time. A step series - it only ever moves down. Each
// step remembers which program set it, for the tooltip.
function recordSeries(history, c, key) {
  const runs = history.filter(b => b.c.id === c.id && boardKey(b) === key)
    .sort((a, b) => a.ts - b.ts);
  const points = [];
  let best = Infinity;
  for (const b of runs) {
    if (b.score < best) {
      best = b.score;
      points.push({ ts: b.ts, score: best, by: b.e.impl_name });
    }
  }
  // Extend the line to the latest measurement, so "still the record" shows.
  const last = runs[runs.length - 1];
  if (points.length && last && last.ts > points[points.length - 1].ts)
    points.push({ ts: last.ts, score: best, by: points[points.length - 1].by, ghost: true });
  return { name: 'record', record: true, points };
}

// ── charts ───────────────────────────────────────────────────────
// Keep charts light no matter how long the log gets: a series keeps its
// first and last points and evenly samples the middle.
function thin(points, cap = 80) {
  if (points.length <= cap) return points;
  const step = (points.length - 1) / (cap - 1);
  return Array.from({ length: cap }, (_, i) => points[Math.round(i * step)]);
}

// Score-over-time line chart for one board: one line per lane, log y-scale
// when the lanes are far apart, a tooltip on every point. `mini` drops the
// axes and labels for the index cards' sparkline.
function historyChart(series, unit, { mini = false } = {}) {
  series = series.map(s => ({ ...s, points: thin(s.points) }));
  const pts = series.flatMap(s => s.points);
  if (!pts.length) return '';
  const W = mini ? 240 : 640, H = mini ? 46 : 170;
  const L = mini ? 2 : 56, R = mini ? 2 : 118, T = mini ? 3 : 10, B = mini ? 3 : 24;
  const t0 = Math.min(...pts.map(p => p.ts)), t1 = Math.max(...pts.map(p => p.ts));
  const v0 = Math.min(...pts.map(p => p.score)), v1 = Math.max(...pts.map(p => p.score));
  const log = v0 > 0 && v1 / v0 > 8;
  const ty = (v) => log ? Math.log(v) : v;
  const [y0, y1] = [ty(v0), ty(v1)];
  const px = (ts) => t1 === t0 ? (L + (W - L - R) / 2) : L + (ts - t0) / (t1 - t0) * (W - L - R);
  const py = (v) => y1 === y0 ? T + (H - T - B) / 2 : T + (y1 - ty(v)) / (y1 - y0) * (H - T - B);
  let frame = '';
  if (!mini) {
    const ticks = [...new Set([v0, v1, log ? Math.exp((y0 + y1) / 2) : (v0 + v1) / 2])];
    frame = ticks.map(v =>
      `<line class="cgrid" x1="${L}" y1="${py(v)}" x2="${W - R}" y2="${py(v)}"/>` +
      `<text class="ctick" x="${L - 6}" y="${py(v) + 3}" text-anchor="end">${fmtScore(v)}</text>`).join('') +
      `<text class="ctick" x="${L}" y="${H - 6}">${fmtWhen(t0)}</text>` +
      (t1 > t0 ? `<text class="ctick" x="${W - R}" y="${H - 6}" text-anchor="end">${fmtWhen(t1)}</text>` : '');
  }
  const lines = series.map(s => {
    // The record line is a step: the old record holds until the moment a
    // better one lands, then drops. Drawn dashed, in the page's ink color.
    if (s.record) {
      const d = s.points.map((p, k) => k === 0
        ? `M${px(p.ts).toFixed(1)},${py(p.score).toFixed(1)}`
        : `H${px(p.ts).toFixed(1)}V${py(p.score).toFixed(1)}`).join('');
      let dots = '', label = '';
      if (!mini) {
        dots = s.points.filter(p => !p.ghost).map(p => {
          const tip = `<title>${esc(p.by)} set the record: ${fmtScore(p.score)} ${esc(unit)} · ${fmtWhen(p.ts)}</title>`;
          return `<circle class="cpt crecord-pt" cx="${px(p.ts).toFixed(1)}" cy="${py(p.score).toFixed(1)}" r="3">${tip}</circle>` +
            `<circle class="chit" cx="${px(p.ts).toFixed(1)}" cy="${py(p.score).toFixed(1)}" r="9" fill="transparent">${tip}</circle>`;
        }).join('');
        const last = s.points[s.points.length - 1];
        label = `<text class="clabel crecord-label" x="${W - R + 8}" y="${(py(last.score) + 3).toFixed(1)}">record</text>`;
      }
      return `<path class="crecord" d="${d}" fill="none" stroke-width="${mini ? 1.5 : 2}"/>${dots}${label}`;
    }
    const ccol = laneColor(s.lane);
    const d = s.points.map((p, k) => `${k ? 'L' : 'M'}${px(p.ts).toFixed(1)},${py(p.score).toFixed(1)}`).join('');
    let dots = '', label = '';
    if (!mini) {
      dots = s.points.map(p => {
        const tip = `<title>${esc(s.name)} · ${fmtScore(p.score)} ${esc(unit)} · ${fmtWhen(p.ts)}</title>`;
        return `<circle class="cpt" cx="${px(p.ts).toFixed(1)}" cy="${py(p.score).toFixed(1)}" r="3" fill="${ccol}">${tip}</circle>` +
          `<circle class="chit" cx="${px(p.ts).toFixed(1)}" cy="${py(p.score).toFixed(1)}" r="9" fill="transparent">${tip}</circle>`;
      }).join('');
      const last = s.points[s.points.length - 1];
      label = `<text class="clabel" x="${W - R + 8}" y="${(py(last.score) + 3).toFixed(1)}" fill="${ccol}">${esc(s.name)}</text>`;
    }
    return `<path d="${d}" fill="none" stroke="${ccol}" stroke-width="${mini ? 1.5 : 2}" stroke-linejoin="round"/>${dots}${label}`;
  }).join('');
  return `<div class="chart${mini ? ' mini' : ''}"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="score history, ${esc(unit)}, lower is better">${frame}${lines}</svg></div>`;
}

// De-collide the right-edge line labels: sort by y, push apart to 12px.
function decollide(html) {
  const div = document.createElement('div');
  div.innerHTML = html;
  for (const svg of div.querySelectorAll('svg')) {
    const labels = [...svg.querySelectorAll('.clabel')].sort((a, b) => +a.getAttribute('y') - +b.getAttribute('y'));
    for (let i = 1; i < labels.length; i++) {
      const prev = +labels[i - 1].getAttribute('y'), cur = +labels[i].getAttribute('y');
      if (cur - prev < 12) labels[i].setAttribute('y', (prev + 12).toFixed(1));
    }
  }
  return div.innerHTML;
}

// ── champions ────────────────────────────────────────────────────
// Current crown holders: for each board of each challenge, the fastest
// ENTERED program - and only when it beats the reference baseline. The
// baseline is the bar to clear, never a competitor; a board it still
// tops has an open crown. Returns [{c, key, row}].
function crowns(S) {
  const out = [];
  for (const c of Object.values(S.challenges))
    for (const [key, rows] of Object.entries(boardsOf(c)))
      if (rows.length && !rows[0].e.is_reference) out.push({ c, key, row: rows[0] });
  return out;
}

// The people leaderboard: who holds crowns, who has proven entries.
// Reference programs are baselines, not entries - they count for nobody.
function championsTable(S) {
  const people = {};
  const P = (h) => people[h] ||= { handle: h, crowns: 0, boards: [], lanes: 0, admitted: 0, bestSpeed: 0 };
  for (const c of Object.values(S.challenges))
    for (const e of c.entries) {
      if (e.is_reference) continue;
      const p = P(e.solver);
      p.lanes++;
      if (e.admitted) p.admitted++;
    }
  for (const { c, key, row } of crowns(S)) {
    const p = P(row.e.solver);
    p.crowns++;
    // Grouped for display: one entry per (challenge, lane), counting boards.
    const held = (p.held ||= {});
    const hk = `${c.id} ${row.e.impl_name}`;
    held[hk] = (held[hk] || 0) + 1;
    const rows = boardsOf(c)[key];
    const spec = rows.find(r => r.e.is_reference);
    if (spec && spec.score > row.score) p.bestSpeed = Math.max(p.bestSpeed, spec.score / row.score);
  }
  for (const p of Object.values(people))
    p.boards = Object.entries(p.held || {}).map(([k, n]) => n > 1 ? `${k} ×${n}` : k);
  return Object.values(people).sort((a, b) => b.crowns - a.crowns || b.admitted - a.admitted);
}
