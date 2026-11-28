import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

function qmlBlocks(source, component) {
  const blocks = [];
  const start = new RegExp(`(^|\\n)\\s*${component}\\s*\\{`, 'g');
  for (const match of source.matchAll(start)) {
    const open = source.indexOf('{', match.index);
    let depth = 0;
    for (let i = open; i < source.length; i++) {
      if (source[i] === '{') depth++;
      else if (source[i] === '}' && --depth === 0) {
        blocks.push(source.slice(open + 1, i));
        break;
      }
    }
  }
  return blocks;
}

const source = fs.readFileSync(new URL('./Model.js', import.meta.url), 'utf8');
const model = {};
vm.createContext(model);
vm.runInContext(source, model, {filename: 'Model.js'});

// The reports `ai-usagebar usage --json` prints, one per shipped provider, as
// wire text rather than reconstructed objects: the panel's contract is this
// JSON, and the rows below are the ones src/sections.rs projects. Kept inline
// because the only fixture directory this file ever read (tests/fixtures) went
// away with the providers it described.
const opencodeGoRaw = `{
  "schema_version": 1,
  "primary": "opencode-go",
  "entries": [
    {
      "id": "opencode-go",
      "name": "OpenCode Go",
      "display_name": "OpenCode Go",
      "short_name": "ocg",
      "icon": "ocg",
      "plan": "OpenCode Go",
      "error": null,
      "stale": false,
      "fetched_at": "2026-08-14T12:00:00Z",
      "sections": [
        {"type": "metric", "label": "Rolling (5h)", "percent": 42, "value": "42%",
         "detail": "37% elapsed · 5pts under", "headline": "percent", "severity": "low",
         "reset_at": "2026-08-14T14:30:00Z", "window_secs": 18000},
        {"type": "metric", "label": "Weekly (7d)", "percent": 66, "value": "66%",
         "detail": "55% elapsed · 11pts ahead", "headline": "percent", "severity": "mid",
         "reset_at": "2026-08-18T00:00:00Z", "window_secs": 604800},
        {"type": "metric", "label": "Monthly", "percent": 81, "value": "81%",
         "detail": "Resets in 17d 12h · Sep 1 00:00", "headline": "percent",
         "severity": "high", "reset_at": "2026-09-01T00:00:00Z", "window_secs": 2592000}
      ]
    }
  ]
}`;

const openrouterRaw = `{
  "schema_version": 1,
  "primary": "openrouter",
  "entries": [
    {
      "id": "openrouter",
      "name": "OpenRouter",
      "display_name": "OpenRouter",
      "short_name": "opr",
      "icon": "󱙺",
      "plan": "OpenRouter",
      "error": null,
      "sections": [
        {"type": "spacer"},
        {"type": "metric", "label": "Credit balance", "percent": 25, "value": "25%",
         "detail": "$25.00 of $100.00 used (25%)", "headline": "percent",
         "severity": "low", "reset_at": null},
        {"type": "spacer"},
        {"type": "block", "label": "Usage by period",
         "body": ["today $1.20 · week $6.40 · month $25.00"]},
        {"type": "spacer"},
        {"type": "block", "label": "Per-key limit", "body": ["$50.00 of $60.00 remaining"]},
        {"type": "spacer"},
        {"type": "block", "label": "Tier", "body": ["paid tier"]},
        {"type": "spacer"},
        {"type": "block", "label": "Recent models", "body": ["openrouter/auto"]}
      ]
    }
  ]
}`;

const deepseekRaw = `{
  "schema_version": 1,
  "primary": "deepseek",
  "entries": [
    {
      "id": "deepseek",
      "name": "DeepSeek",
      "display_name": "DeepSeek",
      "short_name": "dsk",
      "icon": "󰧑",
      "plan": "DeepSeek",
      "error": null,
      "stale": true,
      "fetched_at": "2026-08-14T11:50:00Z",
      "sections": [
        {"type": "spacer"},
        {"type": "text", "label": "Balance", "value": "$8.42"},
        {"type": "block", "label": "Breakdown",
         "body": ["granted $2.00 · topped-up $10.00"]},
        {"type": "spacer"},
        {"type": "block", "label": "API", "body": ["available"]}
      ]
    }
  ]
}`;

const opencodeGoReport = model.parseReport(opencodeGoRaw);
const openrouterReport = model.parseReport(openrouterRaw);
const deepseekReport = model.parseReport(deepseekRaw);
const opencodeGo = opencodeGoReport.entries[0];
const openrouter = openrouterReport.entries[0];
const deepseek = deepseekReport.entries[0];

assert.equal(opencodeGoReport.ok, true);
assert.equal(opencodeGoReport.primary, 'opencode-go');
assert.equal(opencodeGo.id, 'opencode-go');
assert.equal(opencodeGo.status, 'ready');
assert.equal(opencodeGo.stale, false);
assert.equal(opencodeGo.plan, 'OpenCode Go');
assert.equal(opencodeGo.sections.length, 3);
assert.equal(opencodeGo.sections[0].window_secs, 18000);
assert.equal(opencodeGo.sections[1].window_secs, 604800);
assert.equal(opencodeGo.sections[2].window_secs, 2592000);
assert.equal(opencodeGo.sections[0].reset_at, '2026-08-14T14:30:00Z');
// The reset countdown stays in the report for text consumers; the panel draws
// its own live one and only keeps the rest of the detail line.
assert.equal(model.metricDetail(opencodeGo.sections[2]), 'Sep 1 00:00');

assert.equal(openrouterReport.ok, true);
assert.equal(openrouter.id, 'openrouter');
assert.deepEqual(
  Array.from(openrouter.sections, row => row.type),
  ['spacer', 'metric', 'spacer', 'block', 'spacer', 'block', 'spacer', 'block', 'spacer', 'block'],
);
assert.equal(openrouter.sections[3].label, 'Usage by period');
assert.deepEqual(Array.from(openrouter.sections[3].body), ['today $1.20 · week $6.40 · month $25.00']);

assert.equal(deepseekReport.ok, true);
assert.equal(deepseek.id, 'deepseek');
assert.equal(deepseek.stale, true);
assert.equal(deepseek.fetched_at, '2026-08-14T11:50:00Z');
assert.deepEqual(JSON.parse(JSON.stringify(deepseek.sections[1])),
  {type: 'text', label: 'Balance', value: '$8.42'});

// Canonical product names, short codes and glyphs all come from the report.
assert.equal(model.providerName(opencodeGo), 'OpenCode Go');
assert.equal(model.providerName(openrouter), 'OpenRouter');
assert.equal(model.providerName(deepseek), 'DeepSeek');
assert.equal(model.providerShort(opencodeGo), 'ocg');
assert.equal(model.providerShort(openrouter), 'opr');
assert.equal(model.providerShort(deepseek), 'dsk');
assert.equal(model.providerIcon(opencodeGo), 'ocg');
assert.equal(model.providerIcon(openrouter), '󱙺');
assert.equal(model.providerIcon(deepseek), '󰧑');
assert.equal(model.providerIcon({id: 'x'}), '󰚩');

// Marks are asset lookup only: the three shipped providers, and a plain glyph
// for anything else.
assert.equal(model.brandIconFile(opencodeGo), 'opencode.svg');
assert.equal(model.brandIconFile({id: 'opencode-go@work'}), 'opencode.svg');
assert.equal(model.brandIconFile(openrouter), 'openrouter.svg');
assert.equal(model.brandIconFile(deepseek), 'deepseek.svg');
assert.equal(model.brandIconFile({id: 'deepseek@work'}), 'deepseek.svg');
assert.equal(model.brandIconFile({id: 'unknown-vendor'}), '');
assert.equal(model.brandIconFile(null), '');
for (const file of ['opencode.svg', 'openrouter.svg', 'deepseek.svg']) {
  const mark = fs.readFileSync(new URL('./icons/' + file, import.meta.url), 'utf8');
  assert.match(mark, /viewBox="0 0 24 24"/, file);
  assert.match(mark, /<path\s/, file);
  // BrandMark colorizes by luminance, so the source artwork must be white.
  assert.match(mark, /fill="#ffffff"/, file);
  assert.doesNotMatch(mark, /currentColor/, file);
  assert.doesNotMatch(mark, /<(?:script|image|foreignObject|use)\b|\bhref\s*=|\bon\w+\s*=/i, file);
}
// No two shipped providers may share a mark, and every mark a report can name
// must be on disk.
const marks = ['opencode-go', 'openrouter', 'deepseek'].map(slug => model.brandIconFile({id: slug}));
assert.equal(new Set(marks).size, marks.length);
for (const file of marks)
  assert.ok(fs.existsSync(new URL('./icons/' + file, import.meta.url)), file);

// ---- usage history and the pace chart ----
// Rust collects one sample per fetch per window and reports the window's start
// with them. The panel's only job is the deviation arithmetic: an even burn of
// the window is the zero line, so `dev = p - elapsed * 100`.
const WEEK_END = Date.parse('2026-08-20T00:00:00Z');
const WEEK_START = WEEK_END - 7 * 86400000;
const SPAN = WEEK_END - WEEK_START;

function historyEntry(history) {
  return model.parseReport(JSON.stringify({
    entries: [{id: 'opencode-go', error: null, sections: [], history: history}],
  })).entries[0];
}

const charted = historyEntry({
  weekly: {
    resetsAt: '2026-08-20T00:00:00+00:00',
    start: WEEK_START,
    points: [
      {t: WEEK_START + SPAN * 0.25, p: 40},
      {t: WEEK_START + SPAN * 0.5, p: 70},
    ],
  },
  rolling: {resetsAt: '2026-08-16T20:00:00+00:00', start: WEEK_START, points: []},
});
assert.equal(charted.history.weekly.start, WEEK_START);
assert.equal(charted.history.weekly.points.length, 2);
// A window with no samples is dropped rather than kept as an empty curve.
assert.equal(charted.history.rolling, undefined);

const chart = model.chartSeries(charted);
assert.equal(chart.ready, true);
// Only the cumulative windows are charted: the 5h window empties as requests
// age out, so it has no curve.
assert.deepEqual(Array.from(chart.series, line => line.kind), ['weekly']);
assert.equal(chart.series[0].color, '#4da3ff');
assert.deepEqual(Array.from(chart.series[0].points, p => [p.frac, p.dev]), [[0.25, 15], [0.5, 20]]);
// The projection starts from the last reading and is joined to the window's
// close: 70% in half a window projects to 140%, i.e. +40 pp of deviation.
assert.equal(chart.series[0].last.frac, 0.5);
assert.deepEqual(JSON.parse(JSON.stringify(chart.series[0].projection)), {dev: 40, percent: 140});
// The right-hand label names the projected percentage, never the deviation.
assert.equal(model.chartSeries(charted).series[0].projection.percent, 140);

// Two curves, in report order, with the widget's colors.
const both = historyEntry({
  weekly: {resetsAt: '2026-08-20T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START + SPAN * 0.1, p: 5}, {t: WEEK_START + SPAN * 0.2, p: 9}]},
  monthly: {resetsAt: '2026-09-15T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START + SPAN * 0.1, p: 40}, {t: WEEK_START + SPAN * 0.3, p: 60}]},
});
assert.deepEqual(Array.from(model.chartSeries(both).series, line => line.kind), ['weekly', 'monthly']);
assert.deepEqual(Array.from(model.chartSeries(both).series, line => line.color), ['#4da3ff', '#ffb347']);

// Nothing to plot, no curve: an empty history, a single reading, or a provider
// that reports a balance instead of a window.
assert.equal(model.chartSeries(historyEntry(undefined)).ready, false);
assert.equal(model.chartSeries(historyEntry({
  weekly: {resetsAt: '2026-08-20T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START + SPAN * 0.5, p: 70}]},
})).ready, false);
assert.equal(model.chartSeries(openrouter).ready, false);
assert.equal(model.chartSeries(deepseek).ready, false);
assert.equal(model.chartSeries(null).series.length, 0);

// Below 2% of the window a projection is noise, and a window that is exactly
// at zero has nothing to extrapolate.
const tooEarly = model.chartSeries(historyEntry({
  weekly: {resetsAt: '2026-08-20T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START + SPAN * 0.005, p: 1}, {t: WEEK_START + SPAN * 0.01, p: 2}]},
}));
assert.equal(tooEarly.ready, true);
assert.equal(tooEarly.series[0].projection, null);

// Samples from the previous period are not part of this window.
const stale = model.chartSeries(historyEntry({
  weekly: {resetsAt: '2026-08-20T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START - 60000, p: 90}, {t: WEEK_START + SPAN * 0.2, p: 9}]},
}));
assert.equal(stale.ready, false, 'one sample inside the window is not a curve');

// The cap is the widget's 200 newest samples, and a hostile document cannot
// make the panel walk a million points.
const many = historyEntry({
  weekly: {resetsAt: '2026-08-20T00:00:00+00:00', start: WEEK_START,
    points: Array.from({length: 250}, (_, i) => ({t: WEEK_START + i, p: i % 100}))},
});
assert.equal(many.history.weekly.points.length, 200);
assert.equal(many.history.weekly.points[0].t, WEEK_START + 50);
assert.equal(many.history.weekly.points[199].t, WEEK_START + 249);
// Garbage in a window drops that window, never the whole report.
const junk = historyEntry({
  weekly: {resetsAt: 'not-a-date', start: WEEK_START, points: [{t: 1, p: 1}, {t: 2, p: 2}]},
  monthly: {resetsAt: '2026-09-15T00:00:00+00:00', start: WEEK_START,
    points: [{t: WEEK_START + 1, p: 'x'}, {t: WEEK_START + 2, p: 3}]},
});
assert.equal(junk.history.weekly, undefined);
assert.equal(junk.history.monthly.points.length, 1);

// The scale is fixed and comes from one place.
assert.deepEqual({min: model.chartScale().min, max: model.chartScale().max}, {min: -30, max: 30});

// ---- what the bar draws ----
// `auto` keeps the historical highest-percent window; a pin chooses one window
// class by its stated length, and a pin the provider does not offer falls back
// to the highest rather than blanking the bar.
assert.equal(model.headline(opencodeGo).text, '81%');
assert.equal(model.headline(opencodeGo, 'auto').text, '81%');
assert.equal(model.headline(opencodeGo, 'session').text, '42%');
assert.equal(model.headline(opencodeGo, 'session').label, 'Rolling (5h)');
assert.equal(model.headline(opencodeGo, 'weekly').text, '66%');
assert.equal(model.headline(opencodeGo, 'monthly').text, '81%');
assert.equal(model.headline(opencodeGo, 'bogus').text, '81%');
// A balance-only provider has no quota window: every pin keeps the balance.
assert.equal(model.headline(openrouter).text, '25%');
assert.equal(model.headline(openrouter, 'session').text, '25%');
assert.equal(model.headline(openrouter, 'weekly').text, '25%');
assert.equal(model.headline(deepseek).text, '$8.42');
assert.equal(model.headline(deepseek, 'session').text, '$8.42');
assert.equal(model.headline(deepseek, 'monthly').text, '$8.42');
// A metric names which of its two numbers goes on the bar; the label plays no
// part. `percent` is what the report ships for a metered balance.
assert.equal(model.headline(openrouter).severity, 'low');
assert.equal(model.headline(opencodeGo).severity, 'high');
assert.equal(model.headline(opencodeGo).percent, 81);
const freeTier = model.parseReport(JSON.stringify({entries: [{
  id: 'openrouter', error: null,
  sections: [{
    type: 'metric', label: 'Credit balance', percent: 0, value: '$4.10',
    detail: '0% of $0.00 used ($4.10 left)', headline: 'value', severity: 'low',
  }],
}]})).entries[0];
assert.equal(model.headline(freeTier).text, '$4.10');
// An unrecognized word is not a licence to invent a third rendering.
const dollars = model.parseReport(JSON.stringify({entries: [{
  id: 'deepseek', error: null,
  sections: [{
    type: 'metric', label: 'Balance', percent: 60, value: '$2.00',
    detail: '', headline: 'dollars', severity: 'mid',
  }],
}]})).entries[0];
assert.equal(model.headline(dollars).text, '60%');
// A "value" headline with nothing to show falls back rather than blanking.
const emptyValue = model.parseReport(JSON.stringify({entries: [{
  id: 'deepseek', error: null,
  sections: [{
    type: 'metric', label: 'Balance', percent: 60, value: '',
    detail: '', headline: 'value', severity: 'mid',
  }],
}]})).entries[0];
assert.equal(model.headline(emptyValue).text, '60%');
// An older report omits `headline` altogether and stays a percentage.
const legacyMetric = model.parseReport(JSON.stringify({entries: [{
  id: 'opencode-go', sections: [{type: 'metric', label: 'Weekly', percent: 33}],
}]})).entries[0];
assert.equal(model.headline(legacyMetric).text, '33%');
// An entry that never reported a meter keeps a named state.
assert.equal(model.headline({id: 'opencode-go', sections: [], status: 'ready'}).text, 'Ready');
assert.equal(model.headline({id: 'opencode-go', sections: [], status: 'error'}).text, 'Error');
assert.equal(model.headline(null).text, '');

// Alert state follows the highest percent, never the pinned window.
const criticalWeekly = model.parseReport(JSON.stringify({entries: [{
  id: 'opencode-go', error: null,
  sections: [
    {type: 'metric', label: 'Rolling (5h)', percent: 10, value: '10%', detail: '',
     severity: 'low', window_secs: 18000},
    {type: 'metric', label: 'Weekly (7d)', percent: 95, value: '95%', detail: '',
     severity: 'critical', window_secs: 604800},
  ],
}]})).entries[0];
assert.equal(model.headline(criticalWeekly, 'session').text, '10%');
assert.equal(model.headline(criticalWeekly, 'session').severity, 'low');
assert.equal(model.isAlarming(criticalWeekly), true);
assert.equal(model.barChips([criticalWeekly], criticalWeekly, false, true, false, false, false, false, 'session')[0].label, '10%');
assert.equal(model.barChips([criticalWeekly], criticalWeekly, false, true, false, false, false, false, 'session')[0].alarming, true);
assert.equal(model.isAlarming(opencodeGo), false);
assert.equal(model.isAlarming(deepseek), false);
const failedEntry = model.parseReport(JSON.stringify({entries: [
  {id: 'deepseek', status: 'error', error: 'Unavailable', sections: []},
]})).entries[0];
assert.equal(model.isAlarming(failedEntry), false);
assert.equal(model.barChips([failedEntry], failedEntry, false, true, false, false, false, false)[0].label, '!');
assert.equal(model.anyAlarming([failedEntry]), false);
assert.equal(model.anyAlarming([criticalWeekly]), true);

// ---- window classification ----
assert.equal(model.normalizeBarWindow('auto'), 'auto');
assert.equal(model.normalizeBarWindow(''), 'auto');
assert.equal(model.normalizeBarWindow('  Session '), 'session');
assert.equal(model.normalizeBarWindow('5h'), 'session');
assert.equal(model.normalizeBarWindow('rolling'), 'session');
assert.equal(model.normalizeBarWindow('WEEKLY'), 'weekly');
assert.equal(model.normalizeBarWindow('7d'), 'weekly');
assert.equal(model.normalizeBarWindow('Monthly'), 'monthly');
assert.equal(model.normalizeBarWindow('bogus'), 'auto');
assert.equal(model.normalizeBarWindow(undefined), 'auto');
assert.equal(model.normalizeBarWindow('highest'), 'auto');
assert.equal(model.normalizeBarWindow('shortest'), 'session');
for (const option of ['auto', 'session', 'weekly', 'monthly'])
  assert.equal(model.normalizeBarWindow(option), option);

// A stated size is authoritative; an unknown positive size is treated as
// missing so the label decides (an older report predates window_secs).
assert.equal(model.metricMatchesWindow(opencodeGo.sections[0], 'session'), true);
assert.equal(model.metricMatchesWindow(opencodeGo.sections[1], 'weekly'), true);
assert.equal(model.metricMatchesWindow(opencodeGo.sections[1], 'session'), false);
assert.equal(model.metricMatchesWindow({type: 'metric', label: 'Monthly', percent: 5}, 'monthly'), true);
assert.equal(model.metricMatchesWindow({type: 'metric', label: 'Weekly', percent: 5}, 'weekly'), true);
assert.equal(model.metricMatchesWindow({type: 'metric', label: '5-hour', percent: 5}, 'session'), true);
assert.equal(model.metricMatchesWindow({type: 'metric', label: 'Spend (mo)', percent: 5}, 'monthly'), true);
assert.equal(model.metricMatchesWindow({type: 'metric', label: 'Balance', percent: 5}, 'weekly'), false);
assert.equal(model.metricMatchesWindow({type: 'text', label: 'Weekly'}, 'weekly'), false);
// A known 5h/7d size excludes a monthly pin even when the label says monthly.
assert.equal(model.metricMatchesWindow(
  {type: 'metric', label: 'Monthly quota', percent: 90, window_secs: 18000}, 'monthly'), false);
const monthlyOnly = model.parseReport(JSON.stringify({entries: [{
  id: 'opencode-go', error: null,
  sections: [
    {type: 'metric', label: 'Monthly quota', percent: 90, value: '90%', detail: '',
     severity: 'low', window_secs: 18000},
    {type: 'metric', label: 'Monthly dues', percent: 10, value: '10%', detail: '',
     severity: 'low'},
  ],
}]})).entries[0];
assert.equal(model.headline(monthlyOnly, 'monthly').text, '10%');
// Sizes are normalized at the parse boundary: junk is dropped, floats floor.
const secsEdge = model.parseReport(JSON.stringify({entries: [{
  id: 'opencode-go', error: null,
  sections: [
    {type: 'metric', label: 'Zero', percent: 10, window_secs: 0},
    {type: 'metric', label: 'Negative', percent: 20, window_secs: -5},
    {type: 'metric', label: 'String', percent: 30, window_secs: '18000'},
    {type: 'metric', label: 'Float', percent: 40, window_secs: 18000.9},
  ],
}]})).entries[0];
assert.equal(secsEdge.sections[0].window_secs, null);
assert.equal(secsEdge.sections[1].window_secs, null);
assert.equal(secsEdge.sections[2].window_secs, 18000);
assert.equal(secsEdge.sections[3].window_secs, 18000);
assert.equal(model.headline(secsEdge, 'session').text, '40%');

// ---- how percentages read ----
assert.equal(model.normalizeShowAs('left'), 'left');
assert.equal(model.normalizeShowAs(' Remaining '), 'left');
assert.equal(model.normalizeShowAs('used'), 'used');
for (const junk of [undefined, null, '', 'bogus', 42, {}])
  assert.equal(model.normalizeShowAs(junk), 'used', `showAs ${String(junk)} stays used`);
assert.equal(model.shownPercent(18, 'used'), 18);
assert.equal(model.shownPercent(18, 'left'), 82);
assert.equal(model.shownPercent(130, 'left'), 0);
assert.equal(model.shownPercent('x', 'left'), null);
assert.equal(model.percentText(18, 'left'), '82%');
assert.equal(model.percentText(18), '18%');
assert.equal(model.percentText('x', 'left'), '');
// The pinned window and the reading are independent: the pin picks the window,
// the reading only decides which side of it is printed.
assert.equal(model.headline(opencodeGo, 'session', 'left').text, '58%');
assert.equal(model.headline(opencodeGo, 'session', 'left').percent, 42,
  'the percent stays the used share');
assert.equal(model.headline(opencodeGo, 'session', 'left').severity, 'low');
assert.equal(model.headline(freeTier, 'auto', 'left').text, '$4.10',
  'a value headline is not a percentage');

{
  const view = (row, showAs) => {
    const out = model.metricValueView(row, showAs);
    return {text: out.text, left: out.left, percent: out.percent};
  };
  assert.deepEqual(view({percent: 4, value: '4%'}, 'left'), {text: '96%', left: true, percent: 96});
  assert.deepEqual(view({percent: 4, value: '4%'}, 'used'), {text: '4%', left: false, percent: null});
  assert.deepEqual(view({percent: 4, value: ''}, 'left'), {text: '96%', left: true, percent: 96});
  assert.deepEqual(view({percent: 4, value: ''}, 'used'), {text: '4%', left: false, percent: 4});
  assert.deepEqual(view({percent: 30, value: '$7.00'}, 'left'), {text: '$7.00', left: false, percent: null},
    'a value with its own information is kept');
  assert.deepEqual(view({percent: 4, value: '12 / 50 requests'}, 'left'),
    {text: '12 / 50 requests', left: false, percent: null});
  assert.deepEqual(view({percent: 4.4, value: '4.4%'}, 'used'), {text: '4.4%', left: false, percent: null},
    'the used reading keeps the report value untouched');
  assert.deepEqual(view({percent: 100, value: '100%'}, 'left'), {text: '0%', left: true, percent: 0});
  assert.deepEqual(view(null, 'left'), {text: '', left: false, percent: null});
}

// ---- hidden metrics ----
// Keys are the label behind the heading it sits under, so rows that share a
// label under different headings stay distinct; a repeated label is numbered.
assert.deepEqual(Array.from(model.metricKeys(opencodeGo), item => item && item.key),
  ['Rolling (5h)', 'Weekly (7d)', 'Monthly']);
assert.deepEqual(Array.from(model.metricKeys(deepseek), item => item),
  [null, null, null, null, null], 'text and block rows carry no key');
const groupedEntry = model.parseReport(JSON.stringify({entries: [{
  id: 'openrouter@work', error: null,
  sections: [
    {type: 'text', label: 'Weekly', value: ''},
    {type: 'metric', label: 'Credits', percent: 10},
    {type: 'spacer'},
    {type: 'metric', label: 'Credits', percent: 20},
    {type: 'text', label: 'Note', value: 'kept'},
  ],
}]})).entries[0];
assert.deepEqual(Array.from(model.metricKeys(groupedEntry), item => item && item.key),
  [null, 'Weekly / Credits', null, 'Weekly / Credits #2', null]);
assert.deepEqual(Array.from(model.metricKeys(groupedEntry), item => item && item.group),
  [null, 'Weekly', null, 'Weekly', null]);
assert.equal(model.isHeadingRow(groupedEntry.sections[0]), true);
assert.equal(model.isHeadingRow(groupedEntry.sections[4]), false, 'a text row with a value is not a heading');

{
  const clean = model.normalizeHiddenMetrics(JSON.parse(`{
    "opencode-go": ["Monthly", " Monthly ", "", 7],
    "deepseek@work": ["Balance"],
    "empty": [],
    "broken": "Balance",
    "__proto__": ["x"],
    "constructor": ["x"]
  }`));
  assert.deepEqual(Object.keys(clean).sort(), ['deepseek@work', 'opencode-go']);
  assert.deepEqual(Array.from(clean['opencode-go']), ['Monthly', '7']);
  for (const junk of [null, undefined, 'opencode-go', 5, ['opencode-go'], true])
    assert.deepEqual(Object.keys(model.normalizeHiddenMetrics(junk)), []);
  const wide = {};
  for (let i = 0; i < 100; i++) wide['p' + i] = ['k'];
  assert.equal(Object.keys(model.normalizeHiddenMetrics(wide)).length, 64);
  const tall = {'opencode-go': Array.from({length: 100}, (_, i) => 'k' + i)};
  assert.equal(model.normalizeHiddenMetrics(tall)['opencode-go'].length, 32);
  assert.deepEqual(Array.from(model.hiddenKeysFor(clean, 'opencode-go')), ['Monthly', '7']);
  assert.deepEqual(Array.from(model.hiddenKeysFor(clean, 'toString')), []);
  assert.deepEqual(Array.from(model.hiddenKeysFor(clean, 'opencode')), [],
    'an entry id is matched exactly');
}

{
  let hidden = model.toggleHiddenMetric({}, 'opencode-go', 'Monthly');
  assert.deepEqual(Array.from(hidden['opencode-go']), ['Monthly']);
  hidden = model.toggleHiddenMetric(hidden, 'opencode-go', 'Weekly (7d)');
  assert.deepEqual(Array.from(hidden['opencode-go']), ['Monthly', 'Weekly (7d)']);
  hidden = model.toggleHiddenMetric(hidden, 'opencode-go', 'Monthly');
  assert.deepEqual(Array.from(hidden['opencode-go']), ['Weekly (7d)']);
  hidden = model.toggleHiddenMetric(hidden, 'opencode-go', 'Weekly (7d)');
  assert.deepEqual(Object.keys(hidden), []);
  assert.deepEqual(Object.keys(model.toggleHiddenMetric({}, '', 'x')), []);
  assert.deepEqual(Object.keys(model.toggleHiddenMetric({}, 'opencode-go', ' ')), []);
  assert.deepEqual(Object.keys(model.toggleHiddenMetric({}, '__proto__', 'x')), []);
  const before = {'opencode-go': ['Monthly']};
  model.toggleHiddenMetric(before, 'opencode-go', 'Weekly (7d)');
  assert.deepEqual(before['opencode-go'], ['Monthly'], 'the input map is not mutated');
}

{
  // The bar reads the entry without the hidden rows, so the highest window and
  // the alert state never come from a row the user switched off.
  const all = ['Rolling (5h)', 'Weekly (7d)', 'Monthly'];
  assert.equal(model.visibleEntry(opencodeGo, []), opencodeGo, 'nothing hidden returns the entry itself');
  assert.equal(model.visibleEntry(opencodeGo, ['No such metric']), opencodeGo, 'an unknown key hides nothing');
  assert.equal(model.visibleEntry(null, ['x']), null);
  assert.equal(model.visibleEntry(opencodeGo, all), opencodeGo,
    'hiding every metric is ignored, so the bar never goes blank');
  const withoutMonthly = model.visibleEntry(opencodeGo, ['Monthly']);
  assert.equal(Array.from(withoutMonthly.sections, row => row.label).join(','),
    'Rolling (5h),Weekly (7d)');
  assert.equal(model.headline(withoutMonthly).text, '66%');
  assert.equal(model.headline(withoutMonthly, 'monthly').text, '66%',
    'a pinned window that was hidden falls back to what is left');
  assert.equal(opencodeGo.sections.length, 3, 'the original entry keeps every row');
  const hotMonthly = model.parseReport(JSON.stringify({entries: [{id: 'opencode-go', sections: [
    {type: 'metric', label: 'Rolling (5h)', percent: 5, window_secs: 18000},
    {type: 'metric', label: 'Monthly', percent: 100, severity: 'critical'},
  ]}]})).entries[0];
  assert.equal(model.isAlarming(hotMonthly), true);
  assert.equal(model.isAlarming(model.visibleEntry(hotMonthly, ['Monthly'])), false);
  assert.equal(model.barChips([model.visibleEntry(hotMonthly, ['Monthly'])],
    model.visibleEntry(hotMonthly, ['Monthly']), false, true, false, false, false, false)[0].label, '5%');
}

{
  // A hidden metric takes the spacer that leads it when it closes its run of
  // rows; a heading left with no metric under it goes with its spacer.
  const brief = (entry) => Array.from(entry.sections,
    row => row.type === 'spacer' ? '_' : (row.type === 'text' ? 't:' : 'm:') + row.label).join(' ');
  const spaced = model.parseReport(JSON.stringify({entries: [{id: 'opencode-go', sections: [
    {type: 'metric', label: 'Rolling (5h)', percent: 5},
    {type: 'spacer'},
    {type: 'metric', label: 'Weekly (7d)', percent: 90},
  ]}]})).entries[0];
  assert.equal(brief(model.visibleEntry(spaced, ['Weekly (7d)'])), 'm:Rolling (5h)',
    'the spacer that led the hidden row goes too');
  assert.equal(brief(model.visibleEntry(spaced, ['Rolling (5h)'])), '_ m:Weekly (7d)',
    'the spacer before a kept row stays');
  const headed = model.parseReport(JSON.stringify({entries: [{id: 'openrouter@work', sections: [
    {type: 'metric', label: 'Credit balance', percent: 5},
    {type: 'spacer'},
    {type: 'text', label: 'Breakdown', value: ''},
    {type: 'spacer'},
    {type: 'metric', label: 'Credits', percent: 1},
    {type: 'metric', label: 'Promo', percent: 2},
  ]}]})).entries[0];
  assert.equal(brief(model.visibleEntry(headed, ['Breakdown / Promo'])), 'm:Credit balance _ t:Breakdown _ m:Credits',
    'a group that still has a row keeps its heading');
  assert.equal(brief(model.visibleEntry(headed, ['Breakdown / Credits', 'Breakdown / Promo'])),
    'm:Credit balance', 'a heading with no metric left goes with its spacer');
}

{
  const keyOf = (entry, section) => model.metricKeys(entry)[Array.from(entry.sections).indexOf(section)].key;
  const [rolling, weekly, monthly] = Array.from(opencodeGo.sections);
  const can = (hidden, section) => model.canToggleMetric(opencodeGo, hidden, keyOf(opencodeGo, section));
  assert.equal(can({}, rolling) && can({}, weekly) && can({}, monthly), true,
    'with all three on, any one may be hidden');
  const two = {'opencode-go': ['Rolling (5h)', 'Weekly (7d)']};
  assert.equal(can(two, monthly), false, 'the last metric still on cannot be hidden');
  assert.equal(can(two, rolling), true, 'a hidden metric can always be shown again');
  assert.equal(can(two, weekly), true);
  assert.equal(can({'opencode-go': ['Rolling (5h)']}, monthly), true);
  assert.equal(can({'opencode-go@work': ['Rolling (5h)', 'Weekly (7d)']}, monthly), true,
    'another entry id does not count');
  assert.equal(model.canToggleMetric(opencodeGo, {}, ''), false);
  assert.equal(model.canToggleMetric(opencodeGo, {}, null), false);
  assert.equal(model.canToggleMetric(null, {}, 'Monthly'), false);
  const single = model.parseReport(JSON.stringify({entries: [{id: 'openrouter', sections: [
    {type: 'metric', label: 'Credit balance', percent: 25},
  ]}]})).entries[0];
  assert.equal(model.canToggleMetric(single, {}, 'Credit balance'), false,
    'a lone metric has no switch to flip');
}

{
  // What the Settings page offers: one row per metric, its switch locked only
  // when it is the last one on.
  const rows = (entry, hidden) => Array.from(model.metricChoices(entry, hidden),
    row => `${row.label}|${row.checked}|${row.canToggle}`);
  assert.deepEqual(rows(opencodeGo, {}),
    ['Rolling (5h)|true|true', 'Weekly (7d)|true|true', 'Monthly|true|true']);
  assert.deepEqual(rows(opencodeGo, {'opencode-go': ['Weekly (7d)']}),
    ['Rolling (5h)|true|true', 'Weekly (7d)|false|true', 'Monthly|true|true']);
  assert.deepEqual(rows(opencodeGo, {'opencode-go': ['Rolling (5h)', 'Weekly (7d)']}),
    ['Rolling (5h)|false|true', 'Weekly (7d)|false|true', 'Monthly|true|false'],
    'the last metric on has its switch locked');
  assert.deepEqual(Array.from(model.metricChoices(deepseek, {})), [],
    'a balance-only entry has no rows to offer');
  assert.deepEqual(Array.from(model.metricChoices(null, {})), []);
  const groupedRows = model.metricChoices(groupedEntry, {});
  assert.deepEqual(Array.from(groupedRows, row => row.key), ['Weekly / Credits', 'Weekly / Credits #2']);
  assert.equal(groupedRows[0].group, 'Weekly');
  // Flipping a row is the same toggle the panel uses.
  assert.deepEqual(Array.from(model.toggleMetricChoice({}, opencodeGo, 'Monthly')['opencode-go']),
    ['Monthly']);
  assert.deepEqual(Object.keys(model.toggleMetricChoice({'opencode-go': ['Monthly']}, opencodeGo, 'Monthly')),
    [], 'switching it back clears the list');
  // A key the entry has no row for is stored but hides nothing — the panel only
  // ever offers keys `metricChoices` produced.
  assert.deepEqual(Object.keys(model.toggleMetricChoice({}, deepseek, 'Balance')), ['deepseek']);
  assert.equal(model.visibleEntry(deepseek, ['Balance']), deepseek);
  assert.deepEqual(Object.keys(model.toggleMetricChoice({}, null, 'Monthly')), []);
}

// ---- chips, labels and the strip ----
{
  const entries = [opencodeGo, openrouter, deepseek];
  const strip = model.barChips(entries, opencodeGo, true, true, false, false, false, false);
  assert.equal(strip.length, 3);
  assert.deepEqual(Array.from(strip, chip => chip.id), ['opencode-go', 'openrouter', 'deepseek']);
  assert.deepEqual(Array.from(strip, chip => chip.brand),
    ['opencode.svg', 'openrouter.svg', 'deepseek.svg']);
  assert.deepEqual(Array.from(strip, chip => chip.label), ['81%', '25%', '$8.42']);
  assert.deepEqual(Array.from(strip, chip => chip.icon), ['ocg', '󱙺', '󰧑']);
  assert.deepEqual(Array.from(strip, chip => chip.labelOnly), [false, false, false]);
  assert.deepEqual(Array.from(strip, chip => chip.tag), ['ocg', 'opr', 'dsk']);
  assert.deepEqual(Array.from(strip, chip => chip.alarming), [false, false, false]);
  // One entry selected: a lone chip is drawn from the selected entry only.
  const one = model.barChips(entries, openrouter, false, true, false, false, false, false);
  assert.equal(one.length, 1);
  assert.equal(one[0].id, 'openrouter');
  assert.equal(one[0].brand, 'openrouter.svg');
  // With no brand icons the short code leads the value on each chip, because
  // there is no mark to say which provider the number belongs to.
  const plain = model.barChips(entries, opencodeGo, true, true, false, false, false, false, 'auto', 'used', false);
  assert.deepEqual(Array.from(plain, chip => chip.brand), ['', '', '']);
  assert.deepEqual(Array.from(plain, chip => chip.icon), ['', '', '']);
  assert.deepEqual(Array.from(plain, chip => chip.labelOnly), [true, true, true]);
  assert.deepEqual(Array.from(plain, chip => chip.label), ['ocg 81%', 'opr 25%', 'dsk $8.42']);
  // …while a lone chip keeps the icon box and drops the code.
  const plainOne = model.barChips(entries, opencodeGo, false, true, false, false, false, false, 'auto', 'used', false);
  assert.equal(plainOne[0].brand, '');
  assert.equal(plainOne[0].icon, '󰚩');
  assert.equal(plainOne[0].label, '81%');
  assert.equal(plainOne[0].labelOnly, false);
  // The provider tag is opt-in and rides in front of the value.
  const tagged = model.barChips(entries, opencodeGo, true, true, true, false, false, false, 'auto', 'used', false);
  assert.deepEqual(Array.from(tagged, chip => chip.label), ['ocg 81%', 'opr 25%', 'dsk $8.42'],
    'the tag is already leading the label, so it is not repeated');
  const taggedMark = model.barChips(entries, opencodeGo, true, true, true, false, false, false, 'auto', 'used', true);
  assert.deepEqual(Array.from(taggedMark, chip => chip.label), ['ocg 81%', 'opr 25%', 'dsk $8.42']);
  assert.deepEqual(Array.from(taggedMark, chip => chip.labelOnly), [false, false, false],
    'the marks keep their own box');
  // `showValue` off: the tag alone when it is on, the mark alone when it is not.
  assert.deepEqual(
    Array.from(model.barChips(entries, opencodeGo, true, false, true, false, false, false, 'auto', 'used', true),
      chip => chip.label), ['ocg', 'opr', 'dsk']);
  assert.deepEqual(
    Array.from(model.barChips(entries, opencodeGo, true, false, false, false, false, false, 'auto', 'used', true),
      chip => chip.label), ['', '', '']);
  // Placeholders: loading, empty and vertical bars have nothing to click.
  assert.equal(model.barChips([], null, false, true, false, true, false, false)[0].label, '…');
  assert.equal(model.barChips([], null, false, true, false, false, true, false)[0].icon, '󰅙');
  assert.equal(model.barChips([], null, false, true, false, false, true, false)[0].id, undefined);
  assert.equal(model.barChips([], null, false, true, false, false, false, false)[0].id, undefined);
  assert.equal(model.barChips([], null, false, true, false, false, true, true)[0].id, undefined);
  assert.equal(model.barChips(entries, opencodeGo, true, true, false, false, false, true)[0].label, '');
  // A selected entry that is hidden falls back to the first visible one.
  assert.equal(model.barChips([openrouter, deepseek], opencodeGo, false, true, false, false, false, false)[0].id,
    'openrouter');
  assert.equal(model.barChips([opencodeGo, criticalWeekly], opencodeGo, true, true, false, false, false, false)[1].alarming,
    true, 'a chip alarms on its own entry');
}

{
  const entries = [opencodeGo, openrouter, deepseek];
  assert.equal(model.barStrip(entries, false, false, true, false, false), 'ocg  81%  󱙺  25%  󰧑  $8.42');
  assert.equal(model.barStrip(entries, false, false, true, true, false), 'ocg  ocg 81%  󱙺  opr 25%  󰧑  dsk $8.42');
  assert.equal(model.barStrip(entries, false, false, false, false, false), 'ocg  󱙺  󰧑');
  assert.equal(model.barStrip(entries, false, false, true, false, false, 'session'),
    'ocg  42%  󱙺  25%  󰧑  $8.42');
  assert.equal(model.barStrip(entries, false, false, true, false, false, 'bogus'), 'ocg  81%  󱙺  25%  󰧑  $8.42');
  assert.equal(model.barStrip(entries, true, false, true, false, false), 'ocg  81%  󱙺  25%  󰧑  $8.42',
    'the alarming flag only paints the empty and vertical placeholders');
  assert.equal(model.barStrip([], false, false, true, false, false), '󰚩');
  assert.equal(model.barStrip([], true, false, true, false, false), '󰅙');
  assert.equal(model.barStrip([], true, true, true, false, false), '󰅙');
  assert.equal(model.barStrip([], false, false, true, false, true), '󰚩  …');
  assert.equal(model.barStrip(null, false, false, true, false, false), '󰚩');
  assert.equal(model.barChip(opencodeGo, true, true, 'session'), 'ocg  ocg 42%');
  assert.equal(model.barChip(opencodeGo, false, false, 'auto'), 'ocg');
  assert.equal(model.barChip(failedEntry, true, false, 'auto'), '󰚩  !');
  assert.equal(model.barChip(null, true, false, 'auto'), '');
}

// Unlike the strip, the single-entry label carries the icon even when the
// caller passes none, and the provider tag is already resolved by the caller.
assert.equal(model.barLabel(false, false, true, false, true, '29%'), '󰚩  29%');
assert.equal(model.barLabel(false, false, false, false, true, '29%'), '󰚩');
assert.equal(model.barLabel(true, false, true, false, true, '95%'), '󰚩  95%');
assert.equal(model.barLabel(true, false, false, false, false, ''), '󰅙');
assert.equal(model.barLabel(false, true, true, false, true, '29%'), '󰚩');
assert.equal(model.barLabel(true, true, true, false, true, '95%'), '󰅙');
assert.equal(model.barLabel(false, false, true, true, false, ''), '󰚩  …');
assert.equal(model.barLabel(false, false, true, true, true, '29%'), '󰚩  29%',
  'a refresh in progress keeps the last value');
assert.equal(model.barLabel(false, false, true, false, true, '29%', 'gpt'), '󰚩  gpt 29%');
assert.equal(model.barLabel(false, false, false, false, true, '29%', 'gpt'), '󰚩  gpt');
assert.equal(model.barLabel(true, false, true, false, true, '95%', 'ocg'), '󰚩  ocg 95%');
assert.equal(model.barLabel(false, false, true, false, true, '', 'agy'), '󰚩  agy');
assert.equal(model.barLabel(false, true, true, false, true, '29%', 'gpt'), '󰚩');
assert.equal(model.barLabel(false, false, true, true, false, '', 'gpt'), '󰚩  …');
assert.equal(model.barLabel(true, false, true, false, false, '', 'gpt'), '󰅙');
assert.equal(model.barLabel(false, false, true, false, true, '29%', '   '), '󰚩  29%');
assert.equal(model.barLabel(false, false, true, false, true, '29%', undefined), '󰚩  29%');
assert.equal(model.barLabel(false, false, true, false, true, '100%', '', '󱙺'), '󱙺  100%');
assert.equal(model.barLabel(false, false, true, false, true, '<b>100%</b>'), '󰚩  ‹b›100%‹/b›',
  'provider text can never reach Text.AutoText as markup');

// The bar presses a slot's widget by geometry: each chip registers its whole
// column of the slot, the gaps split at their midpoint and the outer columns
// own the button's edge padding, so a press anywhere hits the intended entry.
{
  const chipGaps = (index, count) => {
    const value = model.chipHitGaps(index, count, 10, 8.5);
    return {left: value.left, right: value.right};
  };
  assert.deepEqual(chipGaps(0, 3), {left: 8.5, right: 5});
  assert.deepEqual(chipGaps(1, 3), {left: 5, right: 5});
  assert.deepEqual(chipGaps(2, 3), {left: 5, right: 8.5});
  assert.deepEqual(chipGaps(0, 2), {left: 8.5, right: 5});
  assert.deepEqual(chipGaps(1, 2), {left: 5, right: 8.5});
  assert.deepEqual(chipGaps(0, 1), {left: 8.5, right: 8.5});
  assert.equal([0, 1, 2].map(index => {
    const gaps = chipGaps(index, 3);
    return 40 + gaps.left + gaps.right;
  }).reduce((sum, width) => sum + width, 0), 3 * 40 + 2 * 10 + 17);
  assert.equal(40 + chipGaps(0, 1).left + chipGaps(0, 1).right, 40 + 17);
}

// ---- provider identity and entry selection ----
const parsed = model.parseReport(JSON.stringify({
  primary: 'opencode-go',
  entries: [
    {id: 'opencode-go', display_name: 'OpenCode Go', short_name: 'ocg', plan: 'OpenCode Go',
     error: null, sections: [{type: 'metric', label: 'Weekly (7d)', percent: 20}]},
    {id: 'deepseek@work', display_name: 'DeepSeek · work', short_name: 'dsk', error: null,
     sections: [{type: 'text', label: 'Balance', value: '$3.00'}]},
  ],
}));
assert.equal(parsed.ok, true);
assert.equal(parsed.primary, 'opencode-go');
assert.deepEqual(Array.from(model.filteredEntries(parsed.entries, '')).map(entry => entry.id),
  ['opencode-go', 'deepseek@work']);
assert.deepEqual(Array.from(model.filteredEntries(parsed.entries, 'deepseek')).map(entry => entry.id),
  ['deepseek@work']);
assert.deepEqual(Array.from(model.filteredEntries(parsed.entries, 'opencode-go')).map(entry => entry.id),
  ['opencode-go']);
assert.deepEqual(Array.from(model.filteredEntries(parsed.entries, 'missing')), []);
assert.equal(model.selectedIndex(parsed.entries, 'deepseek@work'), 1);
assert.equal(model.selectedIndex(parsed.entries, 'missing'), 0);
assert.equal(model.selectedIndex([], 'x'), -1);
assert.equal(model.preferredEntryId(parsed.entries, parsed.primary), 'opencode-go');
assert.equal(model.preferredEntryId(parsed.entries, 'deepseek'), 'deepseek@work',
  'a bare slug resolves to the first account of that provider');
assert.equal(model.preferredEntryId(parsed.entries, 'missing'), 'opencode-go');
assert.equal(model.preferredEntryId(parsed.entries, parsed.primary, 'deepseek@work'), 'deepseek@work');
assert.equal(model.preferredEntryId(parsed.entries, parsed.primary, '  DEEPSEEK@WORK  '), 'deepseek@work');
assert.equal(model.preferredEntryId(parsed.entries, parsed.primary, 'missing'), 'opencode-go');
assert.equal(model.preferredEntryId([], 'opencode-go'), '');
assert.equal(model.selectedIndex(parsed.entries, 'OPEnCODE-GO'), 0, 'the current selection is exact');
assert.equal(model.baseProvider('deepseek@work'), 'deepseek');
assert.equal(model.baseProvider(undefined), '');
assert.equal(model.providerShort({id: 'deepseek@work'}), 'deepseek');
assert.equal(model.providerShort({id: 'opencode_go'}), 'opencode-go');
assert.equal(model.providerShort({id: 'x', short_name: '   '}), 'x');
assert.equal(model.providerShort(null), '');
assert.equal(model.providerShort({id: 'x', short_name: '<b>x</b>'}), '‹b›x‹/b›');
assert.equal(model.providerName({id: 'deepseek', display_name: 'DeepSeek · <b>work</b>'}),
  'DeepSeek · ‹b›work‹/b›');
assert.equal(model.providerName({id: 'openrouter', name: 'OpenRouter'}), 'OpenRouter');
assert.equal(model.providerName({id: 'opencode_go'}), 'opencode go');
assert.equal(model.providerName(null), 'AI usage');

// Display-only entry choices. Keep exact ids so named accounts are independent.
{
  const entries = [{id: 'opencode-go', name: 'OpenCode Go', checked: true},
    {id: 'openrouter', name: 'OpenRouter', checked: true},
    {id: 'deepseek@work', name: 'DeepSeek work', checked: true}];
  const ids = rows => Array.from(rows, row => row.id);
  assert.deepEqual(ids(model.barVisibleEntries(entries, null)), ids(entries));
  const hidden = model.toggleBarEntry([], 'opencode-go');
  assert.deepEqual(ids(model.barVisibleEntries(entries, hidden)), ['openrouter', 'deepseek@work']);
  assert.deepEqual(ids(model.barVisibleEntries(entries, model.toggleBarEntry(hidden, 'opencode-go'))), ids(entries));
  assert.deepEqual(ids(model.barVisibleEntries(entries, ['openrouter'])), ['opencode-go', 'deepseek@work']);
  assert.equal(model.barVisibleEntries(entries, ids(entries)).length, 0);
  assert.equal(entries.length, 3, 'display filtering must not remove panel entries');
  assert.deepEqual(Array.from(model.toggleBarEntry(['unavailable-entry'], 'openrouter')),
    ['unavailable-entry', 'openrouter']);
  const saved = model.settingsWithOverrides({id: 'test', unknownSetting: 42}, 'test', {hiddenBarEntries: hidden});
  assert.equal(saved.unknownSetting, 42);
  assert.deepEqual(ids(model.barVisibleEntries(entries, saved.hiddenBarEntries)), ['openrouter', 'deepseek@work']);
  assert.equal(model.providerDisplaySummary(entries), '3/3');
  assert.equal(model.providerDisplaySummary([{...entries[0], checked: false}, entries[1], entries[2]]), '2/3');
  const bar = [{id: 'a'}, {id: 'b'}, {id: 'c'}];
  assert.equal(model.adjacentBarEntryId(bar, 'a', 1), 'b');
  assert.equal(model.adjacentBarEntryId(bar, 'b', -1), 'a');
  assert.equal(model.adjacentBarEntryId(bar, 'c', 1), 'a');
  assert.equal(model.adjacentBarEntryId(bar, 'a', -1), 'c');
  // The bar is showing `a` while the panel selection is a hidden entry.
  assert.equal(model.adjacentBarEntryId(bar, 'hidden', 1), 'b');
  assert.equal(model.adjacentBarEntryId(bar, 'hidden', -1), 'c');
  assert.equal(model.adjacentBarEntryId([], 'a', 1), '');
  assert.equal(model.adjacentBarEntryId(bar, 'a', 0), '');
  assert.equal(model.adjacentBarEntryId(null, 'a', 1), '');
}

// ---- the widget's own settings block ----
{
  const priorWidgetSettings = {
    provider: '', refreshIntervalSec: 90, futureSetting: {keep: true}, id: 'stale-id',
  };
  const selectedWidgetSettings = model.settingsWithSelectedEntry(
    priorWidgetSettings, 'akitaonrails.ai-usagebar', 'deepseek@work');
  assert.deepEqual(JSON.parse(JSON.stringify(selectedWidgetSettings)), {
    id: 'akitaonrails.ai-usagebar',
    provider: '',
    refreshIntervalSec: 90,
    futureSetting: {keep: true},
    lastSelectedEntryId: 'deepseek@work',
  });
  assert.equal(priorWidgetSettings.lastSelectedEntryId, undefined);
  assert.equal(model.settingsWithSelectedEntry({}, 'akitaonrails.ai-usagebar', ''), null);
  assert.equal(model.settingsWithSelectedEntry({}, '', 'deepseek@work'), null);
  const hiddenValueSettings = model.settingsWithOverrides(
    selectedWidgetSettings, 'akitaonrails.ai-usagebar', {showValue: false});
  assert.equal(hiddenValueSettings.showValue, false);
  assert.equal(hiddenValueSettings.lastSelectedEntryId, 'deepseek@work');
  assert.equal(selectedWidgetSettings.showValue, undefined);
  const shownProviderSettings = model.settingsWithOverrides(
    hiddenValueSettings, 'akitaonrails.ai-usagebar', {showProvider: true});
  assert.equal(shownProviderSettings.showProvider, true);
  assert.equal(shownProviderSettings.showValue, false);
  assert.equal(shownProviderSettings.lastSelectedEntryId, 'deepseek@work');
  assert.equal(hiddenValueSettings.showProvider, undefined);
  const protectedSettings = model.settingsWithOverrides({}, 'akitaonrails.ai-usagebar', {
    id: 'wrong-id', constructor: 'ignored', prototype: 'ignored', showValue: false,
  });
  assert.equal(protectedSettings.id, 'akitaonrails.ai-usagebar');
  assert.notEqual(protectedSettings.constructor, 'ignored');
  assert.equal(protectedSettings.prototype, undefined);
  assert.equal(model.settingsWithOverrides({}, '', {}), null);
  assert.equal(model.settingsWithOverrides({}, 'x', null), null);
  assert.equal(model.settingsWithOverrides({}, 'x', ['showValue']), null);
  assert.equal(model.booleanSetting(undefined, true), true);
  assert.equal(model.booleanSetting(false, true), false);
  assert.equal(model.booleanSetting('false', true), false);
  assert.equal(model.booleanSetting('true', false), true);
  assert.equal(model.booleanSetting('invalid', true), true);
  assert.equal(model.booleanSetting(1, false), true);
  assert.equal(model.booleanSetting('off', true), false);
}

// ---- reset countdowns, timestamps and colours ----
// Reset-row fixtures are built from *local* calendar components, not UTC
// strings, so every expectation below is a literal that holds in any timezone
// the panel might run in.
const localReset = (y, mo, d, h, mi) => new Date(y, mo - 1, d, h, mi).toISOString();
const at = (y, mo, d, h, mi) => Date.parse(new Date(y, mo - 1, d, h, mi).toISOString());
assert.equal(model.formatReset(localReset(2026, 8, 14, 22, 0), at(2026, 8, 14, 8, 0)),
  'Resets in 14h 0m · 22:00');
assert.equal(model.formatReset(localReset(2026, 8, 14, 9, 5), at(2026, 8, 14, 8, 0)),
  'Resets in 1h 5m · 09:05');
// Under 24h but past midnight: the date is what stops "03:00" reading as a
// time that already went by this morning.
assert.equal(model.formatReset(localReset(2026, 8, 15, 3, 0), at(2026, 8, 14, 20, 0)),
  'Resets in 7h 0m · Aug 15 03:00');
assert.equal(model.formatReset(localReset(2026, 9, 14, 14, 30), at(2026, 8, 14, 12, 0)),
  'Resets in 31d 2h · Sep 14 14:30');
// Day-of-month is not padded, matching the rest of the row's typography.
assert.equal(model.formatReset(localReset(2026, 9, 5, 14, 0), at(2026, 8, 14, 12, 0)),
  'Resets in 22d 2h · Sep 5 14:00');
assert.equal(model.formatReset('2026-08-14T12:00:00Z', Date.parse('2026-08-14T12:00:00Z')), 'Reset due');
assert.equal(model.formatReset('', Date.parse('2026-08-14T12:00:00Z')), '');
assert.equal(model.formatReset('not-a-date', Date.parse('2026-08-14T12:00:00Z')), '');
assert.equal(model.formatDuration(0), 'now');
assert.equal(model.formatDuration(30_000), '1m');
assert.equal(model.formatDuration(3 * 60 * 60 * 1000), '3h 0m');
assert.equal(model.formatDuration(26 * 60 * 60 * 1000), '1d 2h');
assert.equal(model.pad2(7), '07');
assert.equal(model.isSameLocalDay(new Date(2026, 7, 14, 1, 0), new Date(2026, 7, 14, 23, 0)), true);
assert.equal(model.isSameLocalDay(new Date(2026, 7, 14, 1, 0), new Date(2026, 7, 15, 1, 0)), false);
assert.equal(model.formatUpdated('2026-08-14T12:00:00Z', Date.parse('2026-08-14T12:03:00Z')), 'Updated 3m ago');
assert.equal(model.formatUpdated('2026-08-14T12:00:00Z', Date.parse('2026-08-14T12:00:20Z')), 'Updated just now');
assert.equal(model.formatUpdated('', Date.parse('2026-08-14T12:03:00Z')), 'Updated time unavailable');
assert.equal(model.formatUpdated('nope', Date.parse('2026-08-14T12:03:00Z')), 'Updated time unavailable');

// Theme palette: named Omarchy keys win over the color1–3 aliases.
{
  const namedWins = model.parseThemePalette([
    'color1 = "#111111"',
    'red = "#f7768e"',
    'color2 = "#222222"',
    'green = "#9ece6a"',
    'color3 = "#333333"',
    'yellow = "#e0af68"',
    'orange = "#eb927b"',
  ].join('\n'));
  assert.equal(namedWins.red, '#f7768e');
  assert.equal(namedWins.green, '#9ece6a');
  assert.equal(namedWins.yellow, '#e0af68');
  assert.equal(namedWins.orange, '#eb927b');
  const aliasOnly = model.parseThemePalette([
    'color1 = "#aa1111"',
    'color2 = "#11aa11"',
    'color3 = "#1111aa"',
  ].join('\n'));
  assert.equal(aliasOnly.red, '#aa1111');
  assert.equal(aliasOnly.green, '#11aa11');
  assert.equal(aliasOnly.yellow, '#1111aa');
  assert.equal(aliasOnly.orange, '#aa1111', 'orange falls back to the resolved red');
  const reversedOrder = model.parseThemePalette([
    'red = "#f7768e"',
    'color1 = "#111111"',
  ].join('\n'));
  assert.equal(reversedOrder.red, '#f7768e');
  const defaults = model.parseThemePalette('');
  assert.equal(defaults.red, '#e06c75');
  assert.equal(defaults.green, '#98c379');
}
{
  const palette = {green: '#g', yellow: '#y', orange: '#o', red: '#r'};
  assert.equal(model.severityColor('low', palette), '#g');
  assert.equal(model.severityColor('mid', palette), '#y');
  assert.equal(model.severityColor('high', palette), '#o');
  assert.equal(model.severityColor('critical', palette), '#r');
  assert.equal(model.severityColor('', palette), '#g');
  const defaults = model.severityColor('critical');
  assert.equal(defaults, '#e06c75');
}

// ---- parsing, sanitizing and error text ----
assert.equal(model.parseReport('{').ok, false);
assert.equal(model.parseReport('{}').ok, false);
assert.equal(model.parseReport('{"entries":[{"name":"missing id"}]}').ok, false);
assert.equal(model.parseReport('').ok, false);
assert.equal(model.parseReport('{"entries":[]}').ok, true);
assert.equal(model.parseReport('{"entries":[]}').entries.length, 0);
assert.equal(model.cleanText('bad\u0000value', 20), 'badvalue');
assert.equal(model.cleanText('tab\tcarriage\rC1\u0085value', 40), 'tab carriage C1value');
assert.equal(model.cleanText('😀😀', 3), '😀…');
assert.equal(model.autoTextSafe('<img src="https://example.test/pixel">'),
  '‹img src="https://example.test/pixel"›');
assert.equal(model.autoTextSafe('line\nspoof\u202eright-to-left'), 'line spoofright-to-left');
// The report's own severity wins; a missing one is derived from the percent.
assert.equal(opencodeGo.sections[0].severity, 'low');
assert.equal(model.parseReport(JSON.stringify({entries: [{id: 'x', sections: [
  {type: 'spacer'}, {type: 'metric', label: 'A', percent: 91},
]}]})).entries[0].sections[1].severity, 'critical');
assert.equal(model.parseReport(JSON.stringify({entries: [{id: 'x', sections: [
  {type: 'metric', label: 'A', percent: 40},
]}]})).entries[0].sections[0].percent, 40);
// A section without a usable percent is dropped rather than drawn as zero.
assert.equal(model.parseReport(JSON.stringify({entries: [{id: 'x', sections: [
  {type: 'metric', label: 'Broken'}, {type: 'metric', label: 'Fine', percent: 5},
]}]})).entries[0].sections.length, 1);
assert.equal(model.parseReport(JSON.stringify({entries: [{id: 'x', sections: [
  {type: 'unknown'}, {type: 'spacer'},
]}]})).entries[0].sections.length, 1);
assert.equal(model.normalizeSection(null), null);
assert.equal(model.normalizeEntry(null), null);
assert.equal(model.normalizeEntry({id: '   '}), null);
assert.equal(model.errorMessage(''), 'The usage command failed without an error message.');
assert.equal(model.errorMessage(' boom '), 'boom');

// A missing ai-usagebar binary must be reported as such, with the install
// command, instead of surfacing the helper's raw "not found" text or leaving
// the widget silently stuck on its loading state.
assert.match(model.launchErrorMessage(127, 'env: ai-usagebar: No such file or directory'),
  /ai-usagebar is not installed/);
assert.match(model.launchErrorMessage(127, ''), /omarchy pkg aur add ai-usagebar-bin/);
assert.equal(model.launchErrorMessage(1, 'boom'), 'boom');
assert.equal(model.launchErrorMessage(0, ''), 'The usage command failed without an error message.');

// ---- the settings bridge ----
{
  const settings = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1,
    primary: 'opencode-go',
    primary_choices: [
      {id: 'opencode-go', label: 'OpenCode Go'},
      {id: 'openrouter', label: 'OpenRouter'},
      {id: 'deepseek', label: 'DeepSeek'},
    ],
    keys: [
      {id: 'opencode-go', label: 'OpenCode Go', environment: 'OPENCODE_GO_API_KEY',
       secret_label: 'API key', note: 'usage quota', configured: true,
       inline_configured: true, environment_configured: false},
      {id: 'deepseek', label: 'DeepSeek', environment: 'DEEPSEEK_API_KEY'},
    ],
  }));
  assert.equal(settings.ok, true);
  assert.equal(settings.primary, 'opencode-go');
  assert.deepEqual(Array.from(settings.primary_choices, row => row.id),
    ['opencode-go', 'openrouter', 'deepseek']);
  assert.deepEqual(Array.from(settings.primary_choices, row => row.value),
    ['opencode-go', 'openrouter', 'deepseek']);
  assert.equal(settings.keys[0].environment, 'OPENCODE_GO_API_KEY');
  assert.equal(settings.keys[0].secret_label, 'API key');
  assert.equal(settings.keys[0].note, 'usage quota');
  assert.equal(settings.keys[0].inline_configured, true);
  assert.equal(settings.keys[0].environment_configured, false);
  assert.equal(settings.keys[1].configured, false);
  assert.equal(settings.keys[1].label, 'DeepSeek');
  // A primary the snapshot does not offer falls back to the first choice.
  const stalePrimary = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'retired-vendor',
    primary_choices: [{id: 'openrouter', label: 'OpenRouter'}], keys: [],
  }));
  assert.equal(stalePrimary.primary, 'openrouter');
  const noChoices = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go', primary_choices: [], keys: [],
  }));
  assert.equal(noChoices.ok, true);
  assert.equal(noChoices.primary, '');
  assert.equal(model.parseSettingsSnapshot('{').ok, false);
  assert.equal(model.parseSettingsSnapshot(JSON.stringify({schema_version: 2, primary_choices: [], keys: []})).ok, false);
  assert.equal(model.parseSettingsSnapshot(JSON.stringify({schema_version: 1, primary_choices: [], keys: {}})).ok, false);
  // An unknown id can never reach `settings apply`, and the parser does not
  // invent labels of its own.
  const ids = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go',
    primary_choices: [{id: 'OpenCode Go', label: 'x'}, {id: 'dlab', label: ''}], keys: [],
  }));
  assert.deepEqual(Array.from(ids.primary_choices, row => row.id), ['dlab']);
  assert.equal(ids.primary_choices[0].label, 'dlab');
  assert.equal(ids.primary, 'dlab');
  // Secrets never come back out of the snapshot, only their presence.
  const keyOnly = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'deepseek',
    primary_choices: [{id: 'deepseek', label: 'DeepSeek'}],
    keys: [{id: 'deepseek', value: 'sk-should-not-appear', api_key: 'sk-should-not-appear'}],
  }));
  assert.equal(JSON.stringify(keyOnly).includes('sk-should-not-appear'), false);
  assert.equal(model.settingsId('opencode-go'), 'opencode-go');
  assert.equal(model.settingsId('opencode_go'), 'opencode_go');
  assert.equal(model.settingsId('__proto__'), '');
  assert.equal(model.settingsId('OpenCode Go'), '');
  assert.equal(model.settingsId(''), '');
}

{
  const patch = model.buildSettingsPatch('opencode-go', [
    {id: 'deepseek', action: 'set', value: 'secret-value'},
    {id: 'openrouter', action: 'clear'},
  ]);
  assert.equal(patch.ok, true);
  assert.deepEqual(JSON.parse(patch.payload), {
    schema_version: 1,
    primary: 'opencode-go',
    keys: {
      deepseek: {action: 'set', value: 'secret-value'},
      openrouter: {action: 'clear'},
    },
  });
  const keyOnlyPatch = model.buildSettingsPatch('', [{id: 'deepseek', action: 'clear'}]);
  assert.deepEqual(JSON.parse(keyOnlyPatch.payload), {
    schema_version: 1, keys: {deepseek: {action: 'clear'}},
  });
  assert.equal(model.buildSettingsPatch('', []).ok, false);
  assert.equal(model.buildSettingsPatch('', [], []).ok, false);
  assert.equal(model.buildSettingsPatch('OpenCode Go', []).ok, false);
  assert.equal(model.buildSettingsPatch('opencode-go', [{id: '__proto__', action: 'clear'}]).ok, false);
  assert.equal(model.buildSettingsPatch('opencode-go', [{id: 'deepseek', action: 'clear'}, {id: 'deepseek', action: 'clear'}]).ok, false);
  assert.equal(model.buildSettingsPatch('opencode-go', [{id: 'deepseek', action: 'set', value: ''}]).ok, false);
  assert.equal(model.buildSettingsPatch('opencode-go', [{id: 'deepseek', action: 'set', value: 'x'.repeat(16385)}]).ok, false);
  assert.equal(model.buildSettingsPatch('opencode-go', [{id: 'deepseek', action: 'bogus'}]).ok, false);
  assert.equal(model.parseSettingsApplyResult('{"ok":true}'), true);
  assert.equal(model.parseSettingsApplyResult('{"ok":false}'), false);
  assert.equal(model.parseSettingsApplyResult('not json'), false);
}

// Monthly cycle day: the panel derives OpenCode Go's monthly window length
// from it, so it rides in the same patch and is validated here as well.
{
  const cycle = model.buildSettingsPatch('', [], [], 15);
  assert.deepEqual(JSON.parse(cycle.payload), {schema_version: 1, keys: {}, monthly_cycle_day: 15});
  const cleared = model.buildSettingsPatch('', [], [], null);
  assert.deepEqual(JSON.parse(cleared.payload), {schema_version: 1, keys: {}, monthly_cycle_day: null});
  for (const bad of [0, 29, -1, 15.5, '15', true])
    assert.equal(model.buildSettingsPatch('', [], [], bad).ok, false, `monthly cycle day ${String(bad)}`);
  // An unchanged cycle day is not sent at all.
  assert.equal(model.buildSettingsPatch('', []).ok, false);
}

// Provider on/off switches (#244): the snapshot carries every provider's
// enabled state, an older binary's vendor-less snapshot still parses, and the
// patch gains `vendors` only when a toggle is pending.
{
  const vendorSnapshot = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go',
    primary_choices: [{id: 'opencode-go', label: 'OpenCode Go'}],
    keys: [],
    vendors: [
      {id: 'opencode-go', label: 'OpenCode Go', enabled: true},
      {id: 'openrouter', label: 'OpenRouter', enabled: false},
      {id: 'deepseek', label: 'DeepSeek', enabled: true},
      {id: '__proto__', label: 'never', enabled: true},
      {id: 'drifted', label: 'Drifted', enabled: 'yes'},
    ],
  }));
  assert.equal(vendorSnapshot.ok, true);
  assert.equal(vendorSnapshot.vendors.length, 4);
  assert.equal(vendorSnapshot.vendors[0].id, 'opencode-go');
  assert.equal(vendorSnapshot.vendors[0].enabled, true);
  assert.equal(vendorSnapshot.vendors[1].id, 'openrouter');
  assert.equal(vendorSnapshot.vendors[1].enabled, false);
  assert.equal(vendorSnapshot.vendors[2].label, 'DeepSeek');
  // A non-boolean enabled is treated as off, never coerced from a string.
  assert.equal(vendorSnapshot.vendors[3].enabled, false);
  const noVendors = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go',
    primary_choices: [{id: 'opencode-go', label: 'OpenCode Go'}], keys: [],
  }));
  assert.equal(noVendors.ok, true);
  assert.equal(noVendors.vendors.length, 0);

  const togglePatch = model.buildSettingsPatch('', [], [
    {id: 'openrouter', enabled: true},
    {id: 'deepseek', enabled: false},
  ]);
  assert.equal(togglePatch.ok, true);
  assert.deepEqual(JSON.parse(togglePatch.payload), {
    schema_version: 1, keys: {}, vendors: {openrouter: true, deepseek: false},
  });
  // A save with no pending toggle omits `vendors`, so an older binary still
  // accepts a display-only patch (its ApplyRequest denies unknown fields).
  const noTogglePatch = model.buildSettingsPatch('opencode-go', []);
  assert.deepEqual(JSON.parse(noTogglePatch.payload), {
    schema_version: 1, primary: 'opencode-go', keys: {},
  });
  assert.equal(model.buildSettingsPatch('', [{id: 'deepseek', action: 'clear'}], undefined).ok, true);
  assert.equal(model.buildSettingsPatch('', [], [{id: '__proto__', enabled: true}]).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [{id: 'deepseek'}, {id: 'deepseek', enabled: true}]).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [{id: 'deepseek', enabled: true}, {id: 'deepseek', enabled: false}]).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [{id: 'deepseek', enabled: 'on'}]).ok, false);
}

// Quota-threshold notifications ([notifications]): the snapshot carries the
// switch and the threshold, an older binary's snapshot leaves the section
// hidden, and the patch gains the two keys only when something changed.
{
  const notifySnapshot = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go', primary_choices: [], keys: [],
    notify_enabled: false, notify_threshold: 90,
  }));
  assert.equal(notifySnapshot.ok, true);
  // Cross-realm object, so compare fields rather than prototypes.
  assert.equal(notifySnapshot.notify.enabled, false);
  assert.equal(notifySnapshot.notify.threshold, 90);
  const noNotify = model.parseSettingsSnapshot(JSON.stringify({
    schema_version: 1, primary: 'opencode-go', primary_choices: [], keys: [],
  }));
  assert.equal(noNotify.ok, true);
  assert.equal(noNotify.notify, null);
  assert.equal(model.parseSettingsSnapshot('{').notify, null);
  // A malformed value hides the section rather than inventing a threshold the
  // bar is not using.
  for (const bad of [0, 101, '90', 90.5, null, undefined]) {
    const parsedNotify = model.parseSettingsSnapshot(JSON.stringify({
      schema_version: 1, primary: 'opencode-go', primary_choices: [], keys: [],
      notify_enabled: true, notify_threshold: bad,
    }));
    assert.equal(parsedNotify.notify, null, `notify_threshold ${String(bad)}`);
  }
  const notifyPatch = model.buildSettingsPatch('', [], [], undefined, {enabled: false, threshold: 100});
  assert.equal(notifyPatch.ok, true);
  assert.deepEqual(JSON.parse(notifyPatch.payload), {
    schema_version: 1, keys: {}, notify_enabled: false, notify_threshold: 100,
  });
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {threshold: 90}).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {enabled: true, threshold: 0}).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {enabled: true, threshold: 101}).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {enabled: true, threshold: 90.5}).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {enabled: 'yes', threshold: 90}).ok, false);
  assert.equal(model.buildSettingsPatch('', [], [], undefined, {enabled: true, threshold: '90'}).ok, false);
  // Nothing pending anywhere is still refused.
  assert.equal(model.buildSettingsPatch('', [], [], undefined, undefined).ok, false);
  // The notification patch and the other sections travel together.
  const combined = model.buildSettingsPatch('deepseek', [{id: 'openrouter', action: 'clear'}],
    [{id: 'deepseek', enabled: true}], 1, {enabled: true, threshold: 1});
  assert.deepEqual(JSON.parse(combined.payload), {
    schema_version: 1,
    primary: 'deepseek',
    keys: {openrouter: {action: 'clear'}},
    vendors: {deepseek: true},
    monthly_cycle_day: 1,
    notify_enabled: true,
    notify_threshold: 1,
  });
}

// ---- the plugin manifest ----
const manifest = JSON.parse(fs.readFileSync(new URL('../manifest.json', import.meta.url), 'utf8'));
assert.deepEqual(manifest.kinds, ['bar-widget']);
assert.equal(manifest.entryPoints.barWidget, 'omarchy/BarWidget.qml');
assert.equal(manifest.barWidget.defaults.showValue, true);
const showValueSchema = manifest.barWidget.schema.find(row => row.key === 'showValue');
assert.equal(showValueSchema.type, 'boolean');
assert.equal(showValueSchema.defaultValue, true);
// Opt-in, so an existing bar entry that has never seen this key keeps the
// label it has today.
assert.equal(manifest.barWidget.defaults.showProvider, false);
const showProviderSchema = manifest.barWidget.schema.find(row => row.key === 'showProvider');
assert.equal(showProviderSchema.type, 'boolean');
assert.equal(showProviderSchema.defaultValue, false);
assert.equal(manifest.barWidget.defaults.showAll, false);
const showAllSchema = manifest.barWidget.schema.find(row => row.key === 'showAll');
assert.equal(showAllSchema.type, 'boolean');
assert.equal(showAllSchema.defaultValue, false);
assert.equal(manifest.barWidget.defaults.colorCodeUsage, false);
const colorCodeUsageSchema = manifest.barWidget.schema.find(row => row.key === 'colorCodeUsage');
assert.equal(colorCodeUsageSchema.type, 'boolean');
assert.equal(colorCodeUsageSchema.defaultValue, false);
assert.equal(manifest.barWidget.defaults.uiLocale, 'auto');
const uiLocaleSchema = manifest.barWidget.schema.find(row => row.key === 'uiLocale');
assert.equal(uiLocaleSchema.type, 'enum');
assert.deepEqual(uiLocaleSchema.options, ['auto', 'en', 'pt-BR', 'ru', 'ko', 'es']);
assert.equal(uiLocaleSchema.defaultValue, 'auto');
// Pinned window is opt-in display-only: existing shell.json entries without
// the key keep the historical highest-percent label.
assert.equal(manifest.barWidget.defaults.barWindow, 'auto');
const barWindowSchema = manifest.barWidget.schema.find(row => row.key === 'barWindow');
assert.equal(barWindowSchema.type, 'enum');
assert.deepEqual(barWindowSchema.options, ['auto', 'session', 'weekly', 'monthly']);
assert.equal(barWindowSchema.defaultValue, 'auto');
assert.equal(manifest.barWidget.defaults.showAs, 'used');
const showAsSchema = manifest.barWidget.schema.find(row => row.key === 'showAs');
assert.equal(showAsSchema.type, 'enum');
assert.deepEqual(showAsSchema.options, ['used', 'left']);
assert.equal(showAsSchema.defaultValue, 'used');
assert.equal(manifest.barWidget.defaults.brandIcons, true);
assert.equal(manifest.barWidget.schema.find(row => row.key === 'brandIcons').type, 'boolean');
// The manifest offers the three providers and nothing else: a provider name
// table here would fork from Rust's VendorId.
assert.doesNotMatch(JSON.stringify(manifest), /cursor|antigravity|grok|ollama|anthropic|claude/i);
// The normalizer must accept every option the manifest offers, or the
// dropdown would write a value the panel silently ignores.
for (const option of barWindowSchema.options)
  assert.equal(model.normalizeBarWindow(option), option);
for (const option of showAsSchema.options)
  assert.equal(model.normalizeShowAs(option), option);

// ---- QML contracts ----
const barWidgetSource = fs.readFileSync(new URL('./BarWidget.qml', import.meta.url), 'utf8');
const panelSource = fs.readFileSync(new URL('./Panel.qml', import.meta.url), 'utf8');
const settingsViewSource = fs.readFileSync(new URL('./SettingsView.qml', import.meta.url), 'utf8');
const brandMarkSource = fs.readFileSync(new URL('./BrandMark.qml', import.meta.url), 'utf8');

assert.match(barWidgetSource, /^BarWidget\s*\{/m);
for (const method of ['open', 'close', 'toggle', 'closeForPopoutSwitch'])
  assert.match(barWidgetSource, new RegExp(`function\\s+${method}\\s*\\(`));
assert.match(barWidgetSource, /source:\s*Qt\.resolvedUrl\("Panel\.qml"\)/);
assert.match(barWidgetSource, /target\.anchorItem\s*=\s*button/);
assert.match(barWidgetSource, /target\.hostWidget\s*=\s*root/);
assert.match(barWidgetSource, /function refresh\s*\(/);
// The bar presses a slot's widget with no coordinates, so each chip registers
// as its own click target and the bar picks the one under the pointer; a lone
// chip leaves the button as the only target and its press toggles as before.
assert.match(barWidgetSource, /function\s+chipItems\s*\(/);
assert.match(barWidgetSource, /function\s+syncChipTargets\s*\(/);
assert.match(barWidgetSource, /onModelChanged:\s*Qt\.callLater\(root\.syncChipTargets\)/);
assert.match(barWidgetSource, /if\s*\(chips\.length\s*<=\s*1\)\s*return/);
assert.match(barWidgetSource, /Model\.chipHitGaps\(index,\s*chipRepeater\.count,\s*Style\.space\(10\),\s*Style\.spaceReal\(17\)\s*\/\s*2\)/);
assert.match(barWidgetSource, /fixedWidth:\s*root\.bar\s*&&\s*root\.bar\.vertical\s*\?\s*-1\s*:\s*chipRow\.implicitWidth\n/);
assert.match(barWidgetSource, /height:\s*button\.height/);
assert.match(barWidgetSource, /width:\s*chipContent\.implicitWidth\s*\+\s*hitGaps\.left\s*\+\s*hitGaps\.right/);
assert.match(barWidgetSource, /x:\s*chipHit\.hitGaps\.left/);
assert.match(barWidgetSource, /opacity:\s*root\.opened\s*&&\s*root\.panelItem\s*&&\s*modelData\.id\s*\n\s*&&\s*modelData\.id\s*!==\s*root\.panelItem\.selectedEntryId\s*\?\s*0\.45\s*:\s*1/);
assert.match(barWidgetSource, /Behavior on opacity\s*\{\s*\n\s*NumberAnimation\s*\{\s*duration:\s*140/);
assert.match(barWidgetSource, /id:\s*chipContent[\s\S]*?spacing:\s*Style\.space\(4\)/);
assert.doesNotMatch(barWidgetSource, /BrandMark\s*\{[^}]*\}\s*\n\s*Item\s*\{/);
assert.match(barWidgetSource, /function\s+triggerPress\s*\(buttonCode\)/);
assert.match(barWidgetSource, /root\.panelItem\.openEntry\(chipHit\.chip\.id\s*\|\|\s*""\)/);
assert.match(barWidgetSource, /visible:\s*chipHit\.chip\.labelOnly !== true/);
// Classic alarm chrome when colour-coding is off; RAG colours replace it when on.
assert.match(barWidgetSource, /active:\s*!root\.colorCodeUsage\s*&&\s*root\.alarming/);
assert.match(barWidgetSource, /text:\s*root\.alarming\s*\?\s*"󰅙"\s*:\s*"󰚩"/);
assert.match(barWidgetSource, /function nextEntry\(\) \{\n\s*if \(panelItem\) panelItem\.selectAdjacentBarEntry\(1\)/);
assert.match(barWidgetSource, /selectAdjacentBarEntry\(delta < 0 \? 1 : -1\)/);
assert.doesNotMatch(barWidgetSource, /selectEntry\(/);
assert.doesNotMatch(barWidgetSource, /\bIpcHandler\s*\{/);

assert.match(panelSource, /^Panel\s*\{/m);
assert.match(panelSource, /property\s+var\s+anchorItem:\s*null/);
assert.match(panelSource, /property\s+var\s+hostWidget:\s*null/);
assert.match(panelSource, /SettingsView\s*\{/);
assert.match(panelSource, /function\s+openSettings\s*\(/);
assert.match(panelSource, /function\s+openEntry\s*\(/);
assert.match(panelSource, /id:\s*chip\.id/);
assert.match(panelSource, /setting\("lastSelectedEntryId",\s*""\)/);
assert.match(panelSource, /setting\("showValue",\s*true\)/);
assert.match(panelSource, /setting\("showProvider",\s*false\)/);
assert.match(panelSource, /setting\("showAll",\s*false\)/);
assert.match(panelSource, /setting\("colorCodeUsage",\s*false\)/);
assert.match(panelSource, /Model\.normalizeBarWindow\(setting\("barWindow",\s*"auto"\)\)/);
// The pin covers the bar value and its echoes (hero detail, tooltip), while
// panel rows and the alert state keep the highest-percent headline.
assert.match(panelSource, /Model\.headline\(shapedEntry,\s*barWindow,\s*showAs\)/);
assert.match(panelSource, /Model\.headline\(item,\s*barWindow,\s*showAs\)/);
// A failed or cached refresh alone cannot activate the bar, but a report that
// never arrived has nothing else to show there.
assert.match(panelSource, /readonly property bool reportMissing:\s*loadError\s*!==\s*""\s*&&\s*entries\.length\s*===\s*0/);
assert.match(panelSource, /\(showAll\s*\?\s*shownAnyAlarming\(\)\s*:\s*entryIsAlarming\(barEntry\)\)[\s\S]{0,40}reportMissing/);
assert.match(panelSource, /function shownAnyAlarming\(\)/);
assert.doesNotMatch(panelSource, /chipAlarm\s*=\s*rows\[i\]\.status\s*===\s*"error"/);
assert.match(panelSource, /function\s+setBarWindow\s*\(/);
assert.match(panelSource, /onBarWindowRequested/);
assert.match(panelSource, /showProvider\s*\?\s*Model\.providerShort\(barEntry\)\s*:\s*""/);
assert.match(panelSource, /Model\.barStrip\([\s\S]*?barWindow/);
assert.match(panelSource, /barWindow,\s*showAs,\s*brandIcons\)/);
assert.match(panelSource, /Model\.barChips\([\s\S]*?barWindow/);
assert.match(panelSource, /Model\.providerIcon\(barEntry\)/);
assert.match(panelSource, /BrandMark\s*\{/);
assert.match(panelSource, /brand:\s*root\.settingsOpen\s*\|\|\s*!root\.brandIcons\s*\?\s*""\s*:\s*Model\.brandIconFile\(root\.entry\)/);
assert.match(brandMarkSource, /icons\/" \+ root\.brand/);
assert.match(brandMarkSource, /colorization:\s*1/);
assert.match(brandMarkSource, /colorizationColor:\s*root\.foreground/);

// Provider tabs must wrap into additional rows instead of being clipped by
// the panel edge when more providers are configured than fit on one line.
assert.match(panelSource, /Flow\s*\{[\s\S]*?id:\s*providerList/);
assert.match(panelSource, /flow:\s*Flow\.LeftToRight/);
assert.match(panelSource, /height:\s*visible\s*\?\s*implicitHeight\s*:\s*0/);
assert.doesNotMatch(panelSource, /childrenRect\.height/, 'childrenRect feeds the item\'s own geometry back — a binding loop');
assert.match(panelSource, /providerList\.forceLayout\(\)/);
assert.doesNotMatch(panelSource, /orientation:\s*ListView\.Horizontal/);
// The scroll content keeps a hairline of slack on both sides of the
// Flickable's clip edge, because Qt drops the 1px border of a control that
// sits exactly on the clip boundary at fractional device scales.
assert.match(panelSource, /Column\s*\{[\s\S]*?id:\s*column[\s\S]*?x:\s*Style\.spacing\.hairline/);
assert.match(panelSource, /width:\s*panelFlick\.width\s*-\s*Style\.spacing\.hairline\s*\*\s*2/);
// Long settings forms must remain reachable with mouse wheels and touchpads.
assert.match(panelSource, /WheelHandler\s*\{[\s\S]*?acceptedDevices:\s*PointerDevice\.Mouse\s*\|\s*PointerDevice\.TouchPad/);
const touchpadScale = /event\.pixelDelta\.y\s*\*\s*(\d+(?:\.\d+)?)/.exec(panelSource);
assert.equal(Number(touchpadScale?.[1]), 5, 'touchpad scroll covers five times the raw pixel delta');
assert.match(panelSource, /event\.angleDelta\.y\s*\/\s*120\s*\*\s*Style\.space\(/);
// The persisted choice is the source of truth on every entries change: the
// remembered-entry loop must run BEFORE the current-selection early return, so
// a chosen entry that briefly dropped out of a refresh wins once it is back.
const syncSource = panelSource.slice(
  panelSource.indexOf('function syncSelection()'),
  panelSource.indexOf('function restoreRememberedSelection'));
assert.match(syncSource, /for \(var r = 0; r < visibleEntries\.length; r\+\+\)/, 'remembered-entry loop exists');
assert.ok(
  syncSource.indexOf('for (var r = 0') < syncSource.indexOf('for (var i = 0'),
  'remembered-entry check precedes the current-selection early return'
);
// The panel brand mark colour-codes by the summary severity only when the
// toggle is on; off, it keeps the classic binary foreground vs urgent when
// the worst pool is critical.
assert.match(panelSource, /foreground:\s*root\.colorCodeUsage\s*\n\s*\?\s*root\.severityColorOf\(root\.summary\.severity\s*\|\|\s*""\)/);
assert.match(panelSource, /\(\(root\.summary\.severity\s*\|\|\s*""\)\s*===\s*"critical"\s*\n\s*\?\s*root\.urgent\s*:\s*root\.foreground\)/);
assert.doesNotMatch(panelSource, /BrandMark[\s\S]*foreground:\s*root\.alarming\s*\?/m);
assert.match(panelSource, /function\s+persistSelection\s*\(/);
assert.match(panelSource, /Model\.settingsWithOverrides\(root\.settings,\s*root\.moduleName,\s*values\)/);
assert.match(panelSource, /bar\.shell\.updateEntryInline\(root\.moduleName,\s*entry\)/);
assert.match(panelSource, /persistSelection\(selectedEntryId\)/);
assert.match(panelSource, /Model\.barLabel\(/);
assert.match(panelSource, /labelOnly:\s*chip\.labelOnly/);

// The panel draws the entry from the raw hidden-metrics map and resolves the
// entry id itself, so every account stays independent.
assert.match(panelSource, /Model\.visibleEntry\(item,\s*Model\.hiddenKeysFor\(hiddenMetrics,\s*item\.id\)\)/);
assert.match(panelSource, /Model\.normalizeHiddenMetrics\(setting\("hiddenMetrics",\s*\{\}\)\)/);
assert.match(panelSource, /Model\.normalizeShowAs\(setting\("showAs",\s*"used"\)\)/);
assert.match(panelSource, /Model\.booleanSetting\(setting\("brandIcons",\s*true\),\s*true\)/);
assert.match(panelSource, /if \(!Model\.canToggleMetric\(target,\s*hiddenMetrics,\s*key\)\) return/);
assert.match(panelSource, /hiddenMetrics:\s*Model\.toggleMetricChoice\(hiddenMetrics,\s*target,\s*key\)/);
assert.match(panelSource, /Model\.metricChoices\(item,\s*hiddenMetrics\)/);
assert.match(panelSource, /readonly property var valueView:\s*Model\.metricValueView\(row,\s*root\.showAs\)/);
assert.doesNotMatch(panelSource, /metricEye|hideable|hasHideableMetrics|metric\.hidden_hint/, 'rows carry no switch; the choice lives in Settings');
assert.match(panelSource, /function shownAlarming\(\) \{\s*return entryIsAlarming\(shapedEntry\)/, 'the alert reads the entry without hidden metrics');
// Neither the panel nor the settings page carries a provider table: names come
// from the report, and the providers this product ships are the only ones.
for (const qmlSource of [panelSource, settingsViewSource]) {
  assert.doesNotMatch(qmlSource, /Cursor Models|Other Models|Antigravity|Gemini|Grok|Ollama|Copilot/);
  assert.doesNotMatch(qmlSource, /cursorPool|antigravityPool|showCursor|showAntigravity/);
}

// The usage command must stay behind a helper that can emit exit 127 when the
// binary is absent, without opening a shell-injection boundary.
assert.match(panelSource, /command:\s*\["\/usr\/bin\/env",\s*"ai-usagebar",\s*"usage",\s*"--json"\]/);
assert.doesNotMatch(panelSource, /command:\s*\["(?:\/usr\/bin\/)?(?:ba)?sh"/);
assert.match(panelSource, /onExited:\s*function\(exitCode\)/);
// The pace chart draws what Model.chartSeries derives; the QML keeps no
// provider or window table of its own, and says so when there is no history.
assert.match(panelSource, /Model\.chartSeries\(root\.entry\)/);
assert.match(panelSource, /root\.tr\("graphic\.collecting"\)/);
assert.match(panelSource, /Canvas\s*\{/);
assert.match(panelSource, /ctx\.setLineDash\(/);
assert.doesNotMatch(panelSource, /#4da3ff|#ffb347/, 'curve colors live in Model.js');
assert.match(source, /#4da3ff/);
assert.match(source, /#ffb347/);
assert.match(panelSource, /Model\.launchErrorMessage\(/);

assert.match(settingsViewSource, /command:\s*\["ai-usagebar",\s*"settings",\s*"show"\]/);
assert.match(settingsViewSource, /command:\s*\["ai-usagebar",\s*"settings",\s*"apply"\]/);
assert.match(settingsViewSource, /stdinEnabled:\s*true/);
assert.match(settingsViewSource, /write\(root\.pendingPayload\s*\+\s*"\\n"\)/);
assert.match(settingsViewSource, /password:\s*true/);
assert.match(settingsViewSource, /function\s+finishApply\s*\(\)\s*\{[\s\S]*?scrubSecrets\(\)[\s\S]*?if\s*\(applyExitCode/s);
assert.doesNotMatch(settingsViewSource, /command:\s*\[[^\]]*(?:api.?key|secret|pendingPayload)/i);
assert.match(settingsViewSource, /signal\s+showValueRequested\(bool\s+enabled\)/);
assert.match(settingsViewSource, /root\.tr\("toggle\.show_value"\)/);
assert.match(settingsViewSource, /signal\s+showProviderRequested\(bool\s+enabled\)/);
assert.match(settingsViewSource, /root\.tr\("toggle\.show_provider"\)/);
assert.match(settingsViewSource, /signal\s+showAllRequested\(bool\s+enabled\)/);
assert.match(settingsViewSource, /root\.tr\("toggle\.show_all"\)/);
assert.match(settingsViewSource, /signal\s+colorCodeUsageRequested\(bool\s+enabled\)/);
assert.match(settingsViewSource, /root\.tr\("toggle\.color_code"\)/);
assert.match(settingsViewSource, /signal\s+brandIconsRequested\(bool\s+enabled\)/);
assert.match(settingsViewSource, /signal\s+uiLocaleRequested\(string\s+value\)/);
assert.match(settingsViewSource, /signal\s+barWindowRequested\(string\s+value\)/);
assert.match(settingsViewSource, /text:\s*root\.tr\("section\.bar_window"\)/);
for (const window of ['auto', 'session', 'weekly', 'monthly'])
  assert.match(settingsViewSource,
    new RegExp(`value:\\s*"${window}",\\s*label:\\s*root\\.tr\\("bar_window\\.${window}"\\)`));
assert.match(settingsViewSource, /import\s+"I18n\.js"\s+as\s+I18n/);
assert.match(settingsViewSource, /text:\s*root\.tr\("section\.language"\)/);
assert.match(settingsViewSource, /text:\s*root\.tr\("language\.help"\)/);
assert.doesNotMatch(settingsViewSource, /language\.help".*system|Qt\.locale\(\)\.name/);
for (const locale of ['auto', 'en', 'pt-BR', 'ru', 'ko', 'es'])
  assert.match(settingsViewSource,
    new RegExp(`value:\\s*"${locale}",\\s*label:\\s*root\\.tr\\("language\\.${locale}"\\)`));
assert.match(settingsViewSource, /I18n\.displaySecretLabel\(/);
assert.match(settingsViewSource, /I18n\.displayNote\(/);
assert.match(settingsViewSource, /wrapMode:\s*Text\.WordWrap/);
// Env identifier stays on its own elided line; role+note wrap on the next.
assert.match(
  settingsViewSource,
  /modelData\.environment[\s\S]*?elide:\s*Text\.ElideRight[\s\S]*?displaySecretLabel[\s\S]*?displayNote[\s\S]*?wrapMode:\s*Text\.WordWrap/s,
);
// Provider on/off switches (#244): the section lists the snapshot's vendors
// and routes every change through the same stdin patch as the keys.
assert.match(settingsViewSource, /text:\s*root\.tr\("section\.providers"\)/);
assert.match(settingsViewSource, /model:\s*root\.snapshot\.vendors/);
assert.match(settingsViewSource, /function\s+collectVendorToggles\s*\(/);
assert.match(settingsViewSource, /function\s+setVendorOverride\s*\(/);
assert.match(settingsViewSource, /model:\s*root\.snapshot\.keys/);
assert.match(
  settingsViewSource,
  /Model\.buildSettingsPatch\(selectedPrimary,\s*collectChanges\(\),\s*collectVendorToggles\(\)\s*(,\s*.*)?\)/
);
// Quota-threshold notifications ([notifications]): the section is bound to the
// snapshot's notify values and saves through the same stdin patch.
assert.match(settingsViewSource, /text:\s*root\.tr\("section\.notifications"\)/);
assert.match(settingsViewSource, /root\.tr\("notify\.enabled"\)/);
assert.match(settingsViewSource, /root\.notifySettings/);
assert.match(settingsViewSource, /function\s+collectNotifications\s*\(/);
assert.match(settingsViewSource, /collectNotifications\(\)\)/);
assert.match(settingsViewSource, /root\.tr\("credentials\.paste"/);
assert.match(settingsViewSource, /root\.tr\("error\.notify_threshold"\)/);
assert.match(settingsViewSource, /notifyThresholdValid/);
// Monthly cycle day rides in the same patch (OpenCode Go's monthly window).
assert.match(settingsViewSource, /property var monthlyCycleDay/);
assert.match(settingsViewSource, /monthlyCycleDayPending/);
assert.match(settingsViewSource, /text:\s*root\.tr\("section\.monthly_cycle"\)|root\.tr\("monthly_cycle/);
assert.match(settingsViewSource, /barProviderChoices/);
assert.match(settingsViewSource, /badge:\s*Model\.providerDisplaySummary\(root\.barProviderChoices\)/);
assert.match(settingsViewSource, /badge:\s*root\.vendorSummary\(\)/, 'fetching keeps its independent enabled count');
assert.match(settingsViewSource, /property bool fetchingOpen: false/);
assert.match(settingsViewSource, /onClicked:\s*root\.barProviderToggleRequested\(modelData\.id\)/);

// Panel keys still walk every tab, while the bar's chip steps the visible ones.
assert.match(panelSource, /barProviderChoices:\s*visibleEntries\.map/);
assert.match(panelSource, /function selectAdjacentBarEntry\(delta\)/);
assert.match(panelSource, /Model\.adjacentBarEntryId\(barEntries,\s*selectedEntryId,\s*delta\)/);
assert.match(panelSource, /selectEntry\(root\.entryIndex \+ dx\)/);

// Settings sections fold into one open section at a time, and every action is
// reachable from the keyboard.
{
  const settingsForm = settingsViewSource;
  assert.match(settingsForm, /property string openSection:\s*"display"/);
  assert.match(settingsForm, /function toggleSection\(id\) \{\s*openSection = openSection === id \? "" : id\s*\}/);
  assert.equal((settingsForm.match(/^    Disclosure \{/gm) || []).length, 9);
  for (const id of ['display', 'notifications', 'language', 'barWindow', 'showAs', 'metrics', 'primary', 'providers', 'credentials']) {
    assert.match(settingsForm, new RegExp(`onToggled: root\\.toggleSection\\("${id}"\\)`), `${id} folds`);
    assert.match(settingsForm, new RegExp(`visible: root\\.openSection === "${id}"`), `${id} body`);
  }
  assert.doesNotMatch(settingsForm, /providersOpen|credentialsOpen/);
  assert.doesNotMatch(settingsForm, /^    PanelSectionHeader \{\s*\n\s*text: root\.tr\("section\.(display|language|bar_window|show_as|primary|providers|credentials)"\)/m);

  const buttons = qmlBlocks(settingsForm, 'Button');
  assert.equal(buttons.length, 2, 'all settings action buttons are covered');
  for (const button of buttons)
    assert.match(button, /\bfocusable:\s*true\b/, 'every settings Button is in the tab order');
  const panelActionButtons = qmlBlocks(settingsForm, 'PanelActionButton');
  assert.equal(panelActionButtons.length, 1, 'the credential clear action is covered');
  assert.match(panelActionButtons[0], /\bfocusable:\s*true\b/);
  assert.match(panelActionButtons[0], /\btooltipText:/, 'the icon-only action retains an accessible label');
  const disclosures = qmlBlocks(settingsForm, 'component Disclosure: Item');
  assert.equal(disclosures.length, 1);
  assert.match(disclosures[0], /activeFocusOnTab:\s*true/);
  for (const key of ['Return', 'Enter', 'Space'])
    assert.match(disclosures[0], new RegExp(`Keys\\.on${key}Pressed:\\s*disclosure\\.toggled\\(\\)`));
  assert.match(disclosures[0], /onClicked:\s*\{\s*disclosure\.forceActiveFocus\(\)\s*disclosure\.toggled\(\)/);
  assert.equal(qmlBlocks(settingsForm, 'MouseArea').length, 1,
    'new custom pointer targets need explicit keyboard-equivalence coverage');
  assert.match(settingsForm, /focus:\s*visible/);
  assert.match(settingsForm, /Keys\.onEscapePressed:\s*closeRequested\(\)/);
  assert.match(settingsForm, /signal showAsRequested\(string value\)/);
  assert.match(settingsForm, /signal metricToggleRequested\(string entryId, string key\)/);
  assert.match(settingsForm, /enabled: !root\.saving && modelData\.canToggle/);
  assert.match(settingsForm, /opacity: modelData\.canToggle \? 1 : 0\.45/, 'a locked switch looks locked');
  assert.match(settingsForm, /onChanged:\s*function\(value\)\s*\{\s*root\.showAsRequested\(value\)\s*\}/);
  assert.match(panelSource, /onMetricToggleRequested:\s*function\(entryId,\s*key\)\s*\{\s*root\.setMetricShown\(entryId,\s*key\)\s*\}/);
  assert.match(panelSource, /onShowAsRequested:\s*function\(value\)\s*\{\s*root\.setShowAs\(value\)\s*\}/);
  assert.match(panelSource, /persistWidgetSettings\(\{\s*showAs:\s*next\s*\}\)/);
  assert.match(panelSource, /persistWidgetSettings\(\{\s*brandIcons:\s*next\s*\}\)/);
  const displayKeys = ['showValue', 'brandIcons', 'showProvider', 'colorCodeUsage', 'showAll'];
  assert.deepEqual(displayKeys.map(key => manifest.barWidget.defaults[key]), [true, true, false, false, false]);
  assert.deepEqual(manifest.barWidget.schema.filter(row => displayKeys.includes(row.key)).map(row => row.key), displayKeys);
  assert.deepEqual(manifest.barWidget.schema.filter(row => displayKeys.includes(row.key)).map(row => row.defaultValue), [true, true, false, false, false]);
  assert.deepEqual(
    Array.from(settingsForm.matchAll(/label: root\.tr\("toggle\.(\w+)"\)/g), match => match[1]),
    ['show_value', 'brand_icons', 'show_provider', 'color_code', 'show_all']);
}

// Qt 6.12 added a QtQuick type named Color, and it wins over the qs.Commons
// singleton of the same name wherever a bare `Color` is written. Qualifying the
// singleton is the form Omarchy's own plugin documentation asks for.
for (const file of ['Panel.qml', 'BarWidget.qml', 'BrandMark.qml', 'SettingsView.qml']) {
  const qmlSource = fs.readFileSync(new URL(`./${file}`, import.meta.url), 'utf8');
  assert.match(qmlSource, /^import qs\.Commons as Commons$/m, `${file} imports the palette with an alias`);
  assert.doesNotMatch(qmlSource, /(^|[^.\w])Color\.[a-zA-Z]/, `${file} has no bare Color. reference`);
}
assert.match(panelSource, /target:\s*Commons\.Color/);
assert.doesNotMatch(panelSource, /target:\s*Color\s*$/m);

// ---- translations ----
const i18n = {};
vm.createContext(i18n);
vm.runInContext(fs.readFileSync(new URL('./I18n.js', import.meta.url), 'utf8'), i18n, {
  filename: 'I18n.js',
});

// Catalogs must stay in lockstep: a key in English with no locale sibling falls
// back silently to EN in the panel, which is how half-translated UI ships.
// Arrays created inside vm.runInContext live in another realm; copy out
// before deepEqual so Node's strict comparator accepts them.
assert.deepEqual(Array.from(i18n.SUPPORTED), ['en', 'ru', 'pt-BR', 'ko', 'es']);
const enKeys = Object.keys(i18n.MESSAGES.en).sort();
for (const locale of Array.from(i18n.SUPPORTED)) {
  assert.deepEqual(Object.keys(i18n.MESSAGES[locale]).sort(), enKeys, `${locale} catalog keys`);
  for (const key of enKeys) {
    const value = i18n.MESSAGES[locale][key];
    assert.equal(typeof value, 'string', `${locale}.${key} type`);
    assert.ok(value.trim().length > 0, `${locale}.${key} empty`);
  }
  assert.equal(i18n.MONTHS[locale].length, 12, `${locale} months`);
}

assert.equal(i18n.normalizeLocaleTag('auto'), 'auto');
assert.equal(i18n.normalizeLocaleTag(''), 'auto');
assert.equal(i18n.normalizeLocaleTag('en_US.UTF-8'), 'en');
assert.equal(i18n.normalizeLocaleTag('ru_RU'), 'ru');
assert.equal(i18n.normalizeLocaleTag('pt-BR'), 'pt-BR');
assert.equal(i18n.normalizeLocaleTag('pt_BR.UTF-8'), 'pt-BR');
assert.equal(i18n.normalizeLocaleTag('pt_PT'), 'pt-BR');
assert.equal(i18n.normalizeLocaleTag('ko_KR.UTF-8'), 'ko');
assert.equal(i18n.normalizeLocaleTag('es_MX.UTF-8'), 'es');
assert.equal(i18n.normalizeLocaleTag('es-419'), 'es');
assert.equal(i18n.normalizeLocaleTag('de_DE'), '');

assert.equal(i18n.resolveLocale('auto', 'ru_RU.UTF-8'), 'ru');
assert.equal(i18n.resolveLocale('auto', 'pt_BR'), 'pt-BR');
assert.equal(i18n.resolveLocale('auto', 'de_DE'), 'en');
assert.equal(i18n.resolveLocale('pt-BR', 'en_US'), 'pt-BR');
assert.equal(i18n.resolveLocale('ru', 'en_US'), 'ru');
assert.equal(i18n.resolveLocale('auto', 'ko_KR.UTF-8'), 'ko');
assert.equal(i18n.resolveLocale('auto', 'es_AR.UTF-8'), 'es');
assert.equal(i18n.resolveLocale('es', 'en_US'), 'es');

assert.equal(i18n.t('en', 'section.language'), 'LANGUAGE');
assert.equal(i18n.t('ru', 'section.language'), 'ЯЗЫК');
assert.equal(i18n.t('pt-BR', 'section.language'), 'IDIOMA');
assert.equal(i18n.t('ko', 'section.language'), '언어');
assert.equal(i18n.t('es', 'section.language'), 'IDIOMA');
// The notification section is part of the product, in every catalog.
assert.equal(i18n.t('en', 'section.notifications'), 'NOTIFICATIONS');
assert.equal(i18n.t('ru', 'section.notifications'), 'УВЕДОМЛЕНИЯ');
assert.equal(i18n.t('en', 'notify.threshold'), 'Threshold (%)');
assert.equal(
  i18n.t('en', 'language.help'),
  'Language for the panel and settings. Auto matches your system language.',
);
assert.doesNotMatch(i18n.t('en', 'language.help'), /\{system\}|ru_RU|en_US/);
assert.doesNotMatch(i18n.t('ru', 'language.help'), /\{system\}|ru_RU/);
assert.doesNotMatch(i18n.t('pt-BR', 'language.help'), /\{system\}|pt_BR/);
assert.equal(i18n.t('pt-BR', 'status.refresh_failed', {error: 'boom'}),
  'Falha na atualização; mostrando o relatório anterior. boom');
assert.equal(i18n.t('en', 'no.such.key'), 'no.such.key');
// Report copy stays as it arrives; only the framing sentences are translated.
assert.equal(i18n.displayDetail('ru', '$8.42 of $50.00 used (17%)'),
  '$8.42 из $50.00 использовано (17%)');
assert.equal(i18n.displayDetail('pt-BR', '$25.00 of $100.00 used (25%)'),
  '$25.00 de $100.00 usados (25%)');
assert.equal(i18n.displayDetail('ru', '37% elapsed · 5pts under'), '37% elapsed · 5pts under');
assert.equal(i18n.displayDetail('en', ''), '');
assert.equal(i18n.displaySecretLabel('ru', 'API key'), 'API-ключ');
assert.equal(i18n.displaySecretLabel('en', 'Management key'), 'Management key');
assert.equal(i18n.displayNote('ru', 'usage quota'), 'квота использования');
assert.equal(i18n.displayNote('en', 'usage quota'), 'usage quota');
assert.equal(i18n.displayNote('ko', 'usage quota'), '사용 할당량');
assert.equal(i18n.displayNote('en', 'key at example.test/settings'), 'key at example.test/settings');
assert.equal(i18n.displayNote('en', ''), '');
assert.equal(
  i18n.t('ru', 'hero.settings_detail'),
  'Пока вы не нажмёте «Сохранить», ничего не изменится.',
);

const resetAt = '2026-08-14T12:00:00';
const nowMs = Date.parse('2026-08-14T08:00:00');
assert.match(i18n.formatReset(resetAt, nowMs, 'en'), /^Resets in /);
// A reset on another day carries the date; each catalog orders and suffixes it.
const nextDayReset = '2026-10-03T14:00:00';
const dayBeforeMs = Date.parse('2026-10-02T12:00:00');
assert.match(i18n.formatReset(nextDayReset, dayBeforeMs, 'en'), / · Oct 3 14:00$/);
assert.match(i18n.formatReset(nextDayReset, dayBeforeMs, 'pt-BR'), / · out 3 14:00$/);
assert.match(i18n.formatReset(nextDayReset, dayBeforeMs, 'ko'), /^1일 2시간 후 초기화 · 10월 3일 14:00$/);
assert.match(i18n.formatReset(nextDayReset, dayBeforeMs, 'es'), /^Se reinicia en 1d 2h · 3 oct 14:00$/);
assert.match(i18n.formatReset(resetAt, nowMs, 'ru'), /^Сброс через /);
assert.match(i18n.formatReset(resetAt, nowMs, 'pt-BR'), /^Redefine em /);
assert.equal(i18n.formatUpdated('', nowMs, 'pt-BR'), i18n.t('pt-BR', 'updated.unavailable'));
assert.equal(i18n.formatUpdated(new Date(nowMs - 30_000).toISOString(), nowMs, 'ru'),
  i18n.t('ru', 'updated.just_now'));
assert.equal(i18n.monthName('en', 9), 'Oct');
assert.equal(i18n.monthName('ru', 9), 'окт');
assert.equal(i18n.monthName('en', 12), '');

console.log('Omarchy model tests passed');
