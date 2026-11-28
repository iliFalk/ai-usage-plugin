// Pure data shaping for the Omarchy Quattro widget. Keep this file free of
// QML globals so the exact report contract and selection behavior can also be
// exercised by Node in CI.

function cleanText(value, maxLength) {
  var text = value === undefined || value === null ? "" : String(value)
  // The Rust projection already strips terminal controls. This second, cheap
  // boundary keeps hand-authored/older JSON from putting controls in a
  // long-lived shell process.
  text = text.replace(/[\t\r]/g, " ")
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/g, "")
    .replace(/[\u200e\u200f\u202a-\u202e\u2066-\u2069]/g, "")
  var limit = Number(maxLength) || 2048
  if (text.length <= limit) return text
  var end = limit - 1
  var finalCodeUnit = text.charCodeAt(end - 1)
  if (finalCodeUnit >= 0xd800 && finalCodeUnit <= 0xdbff) end--
  return text.slice(0, end) + "…"
}

// Shared Omarchy components use Text.AutoText. Replace angle brackets before
// passing provider-controlled labels into those components so they can never
// be reclassified as rich text (including an image tag with a remote URL).
function autoTextSafe(value) {
  return cleanText(value, 1000)
    .replace(/[\n\u2028\u2029]/g, " ")
    .replace(/</g, "‹")
    .replace(/>/g, "›")
}

function finitePercent(value) {
  var number = Number(value)
  if (!isFinite(number)) return null
  return Math.max(0, Math.min(100, Math.round(number)))
}

function normalizeSection(raw) {
  if (!raw || typeof raw !== "object") return null
  var type = String(raw.type || "")
  if (type === "spacer") return { type: "spacer" }
  if (type === "metric") {
    var percent = finitePercent(raw.percent)
    if (percent === null) return null
    var severity = String(raw.severity || "")
    if (["low", "mid", "high", "critical"].indexOf(severity) < 0)
      severity = percent >= 90 ? "critical" : percent >= 75 ? "high" : percent >= 50 ? "mid" : "low"
    var windowSecs = Math.floor(Number(raw.window_secs))
    if (!isFinite(windowSecs) || windowSecs <= 0) windowSecs = null
    // Which number the bar draws. Anything but an explicit "value" — including
    // a report old enough not to carry the field — leaves it a percentage.
    var headline = raw.headline === "value" ? "value" : "percent"
    return {
      type: "metric",
      label: cleanText(raw.label, 160),
      percent: percent,
      value: cleanText(raw.value, 240),
      detail: cleanText(raw.detail, 1000),
      headline: headline,
      severity: severity,
      reset_at: cleanText(raw.reset_at, 80),
      window_secs: windowSecs
    }
  }
  if (type === "text") {
    return {
      type: "text",
      label: cleanText(raw.label, 160),
      value: cleanText(raw.value, 1000)
    }
  }
  if (type === "block") {
    var body = Array.isArray(raw.body) ? raw.body : []
    var lines = []
    for (var i = 0; i < body.length && i < 24; i++) lines.push(cleanText(body[i], 1000))
    return { type: "block", label: cleanText(raw.label, 160), body: lines }
  }
  return null
}

function normalizeEntry(raw) {
  if (!raw || typeof raw !== "object") return null
  var id = cleanText(raw.id, 180).trim()
  if (id === "") return null
  var sourceSections = Array.isArray(raw.sections) ? raw.sections : []
  var sections = []
  for (var i = 0; i < sourceSections.length && i < 96; i++) {
    var section = normalizeSection(sourceSections[i])
    if (section) sections.push(section)
  }
  var error = raw.error === undefined || raw.error === null ? "" : cleanText(raw.error, 1200)
  return {
    id: id,
    name: cleanText(raw.name, 240),
    display_name: cleanText(raw.display_name, 240),
    short_name: cleanText(raw.short_name, 24),
    icon: cleanText(raw.icon, 8).trim(),
    plan: cleanText(raw.plan, 240),
    status: error !== "" || raw.status === "error" ? "error" : "ready",
    error: error,
    stale: raw.stale === true,
    fetched_at: cleanText(raw.fetched_at, 80),
    sections: sections,
    history: normalizeHistory(raw.history)
  }
}

// ---- Usage history and the pace chart -------------------------------------
//
// The report carries the samples Rust collected per window
// (`history.<window> = {resetsAt, start, points: [{t, p}]}`). The panel turns
// them into deviation from an even burn of the window, in percentage points:
// `dev = p - elapsed * 100`. Window lengths, and which window starts when, are
// Rust's business — `start` travels with each window, so nothing here decides
// what a window is.

var HISTORY_KINDS = ["rolling", "weekly", "monthly"]
// Only the cumulative windows get a curve: the 5h rolling window empties as
// requests age out, so a deviation line for it is noise (the widget's rule).
var CHART_KINDS = ["weekly", "monthly"]
var CHART_COLORS = { weekly: "#4da3ff", monthly: "#ffb347" }
// The widget's fixed scale: zero in the middle, deviations clamped to ±30 pp.
var CHART_Y_MIN = -30
var CHART_Y_MAX = 30
// Below this share of the window a projection is meaningless noise.
var PROJECTION_MIN_FRAC = 0.02
var MAX_HISTORY_POINTS = 200

function normalizeHistoryWindow(raw) {
  if (!raw || typeof raw !== "object") return null
  var start = Number(raw.start)
  var resetsAt = cleanText(raw.resetsAt, 80)
  if (!isFinite(start) || start <= 0 || resetsAt === "") return null
  // Deterministic parse of an absolute instant; no wall clock is read here.
  var end = Date.parse(resetsAt)
  if (!isFinite(end) || end <= start) return null
  var source = Array.isArray(raw.points) ? raw.points : []
  var points = []
  // Only the newest samples matter; starting at the tail also bounds the work
  // an oversized document can cost.
  var first = Math.max(0, source.length - MAX_HISTORY_POINTS)
  for (var i = first; i < source.length; i++) {
    var point = source[i]
    if (!point || typeof point !== "object") continue
    var t = Number(point.t)
    var p = Number(point.p)
    if (!isFinite(t) || !isFinite(p)) continue
    points.push({ t: t, p: p })
  }
  if (points.length === 0) return null
  return { start: start, end: end, resets_at: resetsAt, points: points }
}

function normalizeHistory(raw) {
  if (!raw || typeof raw !== "object") return null
  var history = {}
  var any = false
  for (var i = 0; i < HISTORY_KINDS.length; i++) {
    var window = normalizeHistoryWindow(raw[HISTORY_KINDS[i]])
    if (!window) continue
    history[HISTORY_KINDS[i]] = window
    any = true
  }
  return any ? history : null
}

// The chart's fixed vertical scale, exposed so the QML renderer never keeps its
// own copy of the bounds.
function chartScale() {
  return { min: CHART_Y_MIN, max: CHART_Y_MAX }
}

function clampFraction(value) {
  if (!isFinite(value)) return 0
  return Math.max(0, Math.min(1, value))
}

// Deviation series for the two charted windows of an entry.
//
// Returns `{ ready, series: [{ kind, color, points: [{frac, dev, p}], last,
// projection: {dev, percent} | null }] }`. `ready` is false for anything with
// fewer than two samples in every charted window — a balance provider, an empty
// history, or a single reading — and the panel says "still collecting" instead
// of drawing a line through one point.
function chartSeries(entry) {
  var history = entry && entry.history ? entry.history : null
  var series = []
  if (!history) return { ready: false, series: series }
  for (var i = 0; i < CHART_KINDS.length; i++) {
    var kind = CHART_KINDS[i]
    var window = history[kind]
    if (!window) continue
    var span = window.end - window.start
    if (!(span > 0)) continue
    var points = []
    for (var j = 0; j < window.points.length; j++) {
      var sample = window.points[j]
      // Samples taken before this window began belong to the previous period.
      if (sample.t < window.start) continue
      var frac = clampFraction((sample.t - window.start) / span)
      points.push({ frac: frac, dev: sample.p - frac * 100, p: sample.p })
    }
    // One sample is a dot, not a trend.
    if (points.length < 2) continue
    var last = points[points.length - 1]
    var projection = null
    if (last.frac >= PROJECTION_MIN_FRAC && last.p > 0) {
      // The end-of-window percentage, not the deviation: the label on the
      // right edge reads "81%", not "+21".
      var projected = last.p / last.frac
      if (isFinite(projected))
        projection = { dev: projected - 100, percent: Math.round(projected) }
    }
    series.push({
      kind: kind,
      color: CHART_COLORS[kind],
      points: points,
      last: last,
      projection: projection
    })
  }
  return { ready: series.length > 0, series: series }
}

function parseReport(raw) {
  try {
    var parsed = JSON.parse(String(raw || ""))
    if (!parsed || !Array.isArray(parsed.entries))
      return { ok: false, error: "The usage command returned an unsupported report.", primary: "", entries: [] }
    var entries = []
    for (var i = 0; i < parsed.entries.length && i < 64; i++) {
      var entry = normalizeEntry(parsed.entries[i])
      if (entry) entries.push(entry)
    }
    if (parsed.entries.length > 0 && entries.length === 0)
      return { ok: false, error: "The usage report did not contain a valid provider entry.", primary: "", entries: [] }
    return { ok: true, error: "", primary: cleanText(parsed.primary, 180).trim(), entries: entries }
  } catch (error) {
    return { ok: false, error: "The usage command returned invalid JSON.", primary: "", entries: [] }
  }
}

function baseProvider(id) {
  return String(id || "").split("@")[0]
}

function providerName(entry) {
  if (!entry) return "AI usage"
  // The Rust report owns canonical product names. `name` is the compatible
  // fallback for older binaries; the machine id is only a last resort.
  var title = cleanText(entry.display_name || entry.name, 240).trim()
  if (title === "") title = baseProvider(entry.id).replace(/_/g, " ") || "AI usage"
  return autoTextSafe(title)
}

// The provider tag. `VendorId::short_name` in Rust owns the codes and ships
// them as `short_name`; a binary older than that field has none, so the machine
// id's vendor half stands in rather than a table living here.
function providerShort(entry) {
  if (!entry) return ""
  var code = cleanText(entry.short_name, 24).trim()
  if (code === "") code = baseProvider(entry.id).replace(/_/g, "-")
  return autoTextSafe(code).trim()
}

function providerIcon(entry) {
  if (!entry) return "󰚩"
  var icon = autoTextSafe(cleanText(entry.icon, 8).trim())
  return icon === "" ? "󰚩" : icon
}

function filteredEntries(entries, configuredProvider) {
  var list = Array.isArray(entries) ? entries : []
  var wanted = String(configuredProvider || "").trim().toLowerCase()
  if (wanted === "") return list.slice()
  var exact = list.filter(function(entry) { return String(entry.id).toLowerCase() === wanted })
  if (exact.length > 0) return exact
  return list.filter(function(entry) { return baseProvider(entry.id).toLowerCase() === wanted })
}

function selectedIndex(entries, selectedId) {
  var list = Array.isArray(entries) ? entries : []
  for (var i = 0; i < list.length; i++) if (list[i].id === selectedId) return i
  return list.length > 0 ? 0 : -1
}

function preferredEntryId(entries, primaryProvider, rememberedEntryId) {
  var list = Array.isArray(entries) ? entries : []
  if (list.length === 0) return ""
  var remembered = cleanText(rememberedEntryId, 180).trim().toLowerCase()
  for (var r = 0; r < list.length; r++)
    if (String(list[r].id).toLowerCase() === remembered) return list[r].id
  var primary = String(primaryProvider || "").toLowerCase()
  for (var i = 0; i < list.length; i++)
    if (String(list[i].id).toLowerCase() === primary) return list[i].id
  for (var j = 0; j < list.length; j++)
    if (baseProvider(list[j].id).toLowerCase() === primary) return list[j].id
  return list[0].id
}

function settingsWithOverrides(settings, moduleName, overrides) {
  var moduleId = cleanText(moduleName, 180).trim()
  if (moduleId === "" || !overrides || typeof overrides !== "object" || Array.isArray(overrides))
    return null

  var next = { id: moduleId }
  var current = settings && typeof settings === "object" && !Array.isArray(settings)
    ? settings : {}
  for (var key in current) {
    if (key === "id" || key === "__proto__" || key === "constructor" || key === "prototype")
      continue
    next[key] = current[key]
  }
  for (var overrideKey in overrides) {
    if (overrideKey === "id" || overrideKey === "__proto__" || overrideKey === "constructor"
        || overrideKey === "prototype") continue
    next[overrideKey] = overrides[overrideKey]
  }
  return next
}

function settingsWithSelectedEntry(settings, moduleName, entryId) {
  var selected = cleanText(entryId, 180).trim()
  if (selected === "") return null
  return settingsWithOverrides(settings, moduleName, { lastSelectedEntryId: selected })
}

function booleanSetting(value, fallback) {
  if (value === true || value === false) return value
  var normalized = String(value === undefined || value === null ? "" : value).trim().toLowerCase()
  if (["true", "1", "yes", "on"].indexOf(normalized) >= 0) return true
  if (["false", "0", "no", "off"].indexOf(normalized) >= 0) return false
  return fallback === true
}

// `providerLabel` is already resolved by the caller: empty when the opt-in
// provider switch is off, so the icon-and-value label is unchanged for everyone
// who never turns it on. A vertical bar has no width for either field and
// keeps showing the icon alone.
function barLabel(alarming, vertical, showValue, loading, hasEntry, summaryText,
                  providerLabel, icon) {
  icon = autoTextSafe(icon || "").trim() || "󰚩"
  if (vertical) return alarming ? "󰅙" : icon
  if (loading && !hasEntry) return icon + "  …"
  if (!hasEntry) return alarming ? "󰅙" : icon
  var provider = autoTextSafe(providerLabel).trim()
  var summary = showValue ? autoTextSafe(summaryText).trim() : ""
  if (provider === "") return summary === "" ? icon : icon + "  " + summary
  // One space between tag and value; the wider gap stays next to the icon.
  return summary === "" ? icon + "  " + provider
    : icon + "  " + provider + " " + summary
}

/**
 * Normalizes how a percentage reads: the consumed share ("used") or what is left of the same window ("left").
 * Anything but an explicit "left" stays "used", so an existing shell.json keeps its reading.
 * @param {*} value Raw setting value.
 * @returns {"used"|"left"}
 */
function normalizeShowAs(value) {
  var text = cleanText(value, 16).trim().toLowerCase()
  return text === "left" || text === "remaining" ? "left" : "used"
}

/**
 * Converts a used percentage to the number drawn for the chosen reading.
 * Which window the bar picks never depends on this; only the number drawn for it does.
 * @param {number|string} percent Used percentage from the report.
 * @param {*} showAs Raw reading setting.
 * @returns {number|null} The percentage to draw, or null when the input is not a number.
 */
function shownPercent(percent, showAs) {
  var number = Number(percent)
  if (!isFinite(number)) return null
  return normalizeShowAs(showAs) === "left" ? Math.max(0, 100 - number) : number
}

// Prints a shown percentage with at most one decimal ("70.5%", "70%") so a
// precise figure does not collapse to a whole number. Meters keep integers.
function shownPctText(shown) {
  if (shown === null) return ""
  return String(Math.round(shown * 10) / 10)
}

/**
 * Formats a used percentage as drawn for the chosen reading.
 * @param {number|string} percent Used percentage from the report.
 * @param {*} showAs Raw reading setting.
 * @returns {string} For example "18%", or an empty string when the input is not a number.
 */
function percentText(percent, showAs) {
  var shown = shownPercent(percent, showAs)
  if (shown === null) return ""
  return shownPctText(shown) + "%"
}

var UNSAFE_KEYS = ["__proto__", "constructor", "prototype"]

/**
 * Tells whether an object owns a key itself, ignoring anything inherited.
 * @param {Object} object Object to inspect.
 * @param {string} key Key to look for.
 * @returns {boolean}
 */
function hasOwn(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key)
}

/**
 * Computes the key each metric of an entry is hidden under: its label behind the text heading row above it,
 * so rows with the same label in different groups stay distinct. A label repeated under the same heading is
 * numbered.
 * @param {Object} entry Entry from the report.
 * @returns {Array<{key: string, group: string}|null>} One item per section, null for a section that is not a metric.
 */
function metricKeys(entry) {
  var sections = entry && Array.isArray(entry.sections) ? entry.sections : []
  var keys = []
  var heading = ""
  var seen = {}
  for (var i = 0; i < sections.length; i++) {
    var section = sections[i]
    if (isHeadingRow(section)) {
      heading = cleanText(section.label, 80).trim()
      keys.push(null)
      continue
    }
    if (!section || section.type !== "metric") {
      keys.push(null)
      continue
    }
    var group = heading
    var label = cleanText(section.label, 160).trim()
    var base = group === "" ? label : group + " / " + label
    var count = hasOwn(seen, base) ? seen[base] : 0
    seen[base] = count + 1
    keys.push({ key: count > 0 ? base + " #" + (count + 1) : base, group: group })
  }
  return keys
}

/**
 * Tells whether a section is a group heading: a text row with a label and no value.
 * @param {Object} section Section from the report.
 * @returns {boolean}
 */
function isHeadingRow(section) {
  if (!section || section.type !== "text") return false
  var value = section.value === undefined || section.value === null ? "" : String(section.value)
  return cleanText(section.label, 160).trim() !== "" && value.trim() === ""
}

/**
 * Reads the `hiddenMetrics` setting, a map from an entry id to the metric keys switched off for it.
 * The value is hand-editable and lives in a long-lived shell process, so it is read strictly and kept bounded.
 * @param {*} raw Raw setting value.
 * @returns {Object<string, string[]>} Entry id to hidden metric keys, without empty lists.
 */
function normalizeHiddenMetrics(raw) {
  var out = {}
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return out
  var ids = Object.keys(raw)
  var kept = 0
  for (var i = 0; i < ids.length && kept < 64; i++) {
    var id = cleanText(ids[i], 180).trim()
    if (id === "" || UNSAFE_KEYS.indexOf(id) >= 0) continue
    var list = Array.isArray(raw[ids[i]]) ? raw[ids[i]] : []
    var keys = []
    for (var j = 0; j < list.length && j < 32; j++) {
      var key = cleanText(list[j], 240).trim()
      if (key !== "" && keys.indexOf(key) < 0) keys.push(key)
    }
    if (keys.length > 0) {
      out[id] = keys
      kept++
    }
  }
  return out
}

/**
 * Lists the metric keys switched off for one entry.
 * @param {*} hidden Raw `hiddenMetrics` setting.
 * @param {string} entryId Entry id, matched exactly (so `deepseek@work` and `deepseek` are independent).
 * @returns {string[]}
 */
function hiddenKeysFor(hidden, entryId) {
  var id = cleanText(entryId, 180).trim()
  var map = normalizeHiddenMetrics(hidden)
  return hasOwn(map, id) ? map[id] : []
}

/**
 * Computes the next `hiddenMetrics` after switching one metric of one entry on or off.
 * An entry left with nothing hidden drops out, so the map never carries empty lists. The input is not mutated.
 * @param {*} hidden Raw `hiddenMetrics` setting.
 * @param {string} entryId Entry id.
 * @param {string} key Metric key from `metricKey`.
 * @returns {Object<string, string[]>}
 */
function toggleHiddenMetric(hidden, entryId, key) {
  var next = normalizeHiddenMetrics(hidden)
  var id = cleanText(entryId, 180).trim()
  var metric = cleanText(key, 240).trim()
  if (id === "" || metric === "" || UNSAFE_KEYS.indexOf(id) >= 0) return next
  var list = hasOwn(next, id) ? next[id].slice() : []
  var at = list.indexOf(metric)
  if (at >= 0) list.splice(at, 1)
  else if (list.length < 32) list.push(metric)
  if (list.length === 0) delete next[id]
  else next[id] = list
  return next
}

/**
 * Returns the entry as the bar and the tooltip see it, without the metrics switched off for it, so the
 * highest-percent choice, the alert state and every echo of them never read a row the user hid.
 * A hidden metric takes the spacer that leads it when it closes its run of rows (a heading that shares one spacer keeps
 * it for the next row), and a heading left with no metric under it goes too, with the
 * spacer that precedes it, so no gap is left behind. At least one metric always stays: a hidden list that
 * would remove every metric (a hand-edited shell.json) is ignored, so the bar never goes blank.
 * The settings page still offers the original entry, hidden rows included, so they can be switched back on.
 * @param {Object} entry Entry from the report.
 * @param {string[]} hiddenKeys Metric keys switched off for the entry.
 * @returns {Object} The entry itself when nothing is hidden, otherwise a copy without the hidden metrics.
 */
function visibleEntry(entry, hiddenKeys) {
  var hidden = Array.isArray(hiddenKeys) ? hiddenKeys : []
  if (!entry || hidden.length === 0) return entry
  var sections = Array.isArray(entry.sections) ? entry.sections : []
  var keys = metricKeys(entry)
  var kept = []
  var metrics = 0
  var keptMetrics = 0
  var block = null
  var closeBlock = function() {
    if (block && block.had > 0 && block.kept === 0) {
      var spaced = block.start > 0 && kept[block.start - 1].type === "spacer"
      kept.splice(spaced ? block.start - 1 : block.start, spaced ? 2 : 1)
    }
    block = null
  }
  for (var i = 0; i < sections.length; i++) {
    var section = sections[i]
    if (isHeadingRow(section)) {
      closeBlock()
      block = { start: kept.length, had: 0, kept: 0 }
    }
    if (keys[i]) {
      metrics++
      if (block) block.had++
      if (hidden.indexOf(keys[i].key) >= 0) {
        var after = sections[i + 1]
        var endsRun = !after || after.type === "spacer" || isHeadingRow(after)
        if (endsRun && kept.length > 0 && kept[kept.length - 1].type === "spacer") kept.pop()
        continue
      }
      keptMetrics++
      if (block) block.kept++
    }
    kept.push(section)
  }
  closeBlock()
  if (metrics > 0 && keptMetrics === 0) return entry
  if (metrics === keptMetrics) return entry
  var copy = {}
  for (var field in entry) copy[field] = entry[field]
  copy.sections = kept
  return copy
}

/**
 * Tells whether a metric's switch may be flipped. Showing a hidden metric is always allowed; hiding one is
 * refused when it is the last metric of the entry still on.
 * @param {Object} entry Entry from the report.
 * @param {*} hidden Raw `hiddenMetrics` setting.
 * @param {string} key Metric key from `metricKeys`.
 * @returns {boolean}
 */
function canToggleMetric(entry, hidden, key) {
  if (!entry || key === null || key === undefined || key === "") return false
  var off = hiddenKeysFor(hidden, entry.id)
  if (off.indexOf(key) >= 0) return true
  var visible = 0
  metricKeys(entry).forEach(function(item) {
    if (item && off.indexOf(item.key) < 0) visible++
  })
  return visible > 1
}

function barChip(entry, showValue, showProvider, barWindow, showAs) {
  if (!entry) return ""
  var icon = providerIcon(entry)
  var provider = showProvider ? providerShort(entry) : ""
  var summary = ""
  if (showValue) {
    if (entry.error) summary = "!"
    else summary = autoTextSafe(headline(entry, barWindow, showAs).text).trim()
  }
  if (provider === "") return summary === "" ? icon : icon + "  " + summary
  return summary === "" ? icon + "  " + provider : icon + "  " + provider + " " + summary
}

// Every visible entry as its own icon+value chip. A vertical bar has no
// width for the strip and keeps a single glyph, same as `barLabel`.
function barStrip(entries, alarming, vertical, showValue, showProvider, loading, barWindow, showAs) {
  var list = Array.isArray(entries) ? entries : []
  if (vertical) return alarming ? "󰅙" : "󰚩"
  if (loading && list.length === 0) return "󰚩  …"
  if (list.length === 0) return alarming ? "󰅙" : "󰚩"
  var chips = []
  for (var i = 0; i < list.length; i++) {
    var chip = barChip(list[i], showValue, showProvider, barWindow, showAs)
    if (chip !== "") chips.push(chip)
  }
  return chips.length === 0 ? "󰚩" : chips.join("  ")
}

function anyAlarming(entries) {
  var list = Array.isArray(entries) ? entries : []
  for (var i = 0; i < list.length; i++) {
    if (isAlarming(list[i])) return true
  }
  return false
}

// The mark an entry is drawn with. The report names the provider (its machine
// id) and this file owns the artwork, because each frontend ships its own.
function brandIconFile(entry) {
  return brandFileFor(baseProvider(entry && entry.id))
}

// Official brand marks shipped next to this file. A missing file falls back
// to the nerd-font glyph from the Rust report — the table is asset lookup,
// not a second copy of provider names.
function brandFileFor(provider) {
  switch (provider) {
    case "opencode-go":
      return "opencode.svg"
    case "openrouter":
      return "openrouter.svg"
    case "deepseek":
      return "deepseek.svg"
    default:
      return ""
  }
}

function barChips(entries, selected, showAll, showValue, showProvider, loading, alarming, vertical, barWindow, showAs, brandIcons) {
  var list = Array.isArray(entries) ? entries : []
  if (vertical) {
    return [{ brand: "", icon: alarming ? "󰅙" : "󰚩", label: "", alarming: alarming === true }]
  }
  if (loading && list.length === 0) {
    return [{ brand: "", icon: "󰚩", label: "…", alarming: false }]
  }
  if (list.length === 0) {
    return [{ brand: "", icon: alarming ? "󰅙" : "󰚩", label: "", alarming: alarming === true }]
  }
  var shown = showAll ? list : list.filter(function(entry) { return selected && entry.id === selected.id })
  if (shown.length === 0) shown = [list[0]]
  var chips = []
  var plain = brandIcons === false
  for (var i = 0; i < shown.length; i++) {
    var entry = shown[i]
    var brand = plain ? "" : brandIconFile(entry)
    var labelOnly = brand === "" && !(plain && shown.length === 1)
    var tag = providerShort(entry)
    var label = ""
    if (showProvider && !labelOnly) label = tag
    if (showValue) {
      var summary = entry.error ? "!" : autoTextSafe(headline(entry, barWindow, showAs).text).trim()
      label = label === "" ? summary : (summary === "" ? label : label + " " + summary)
    }
    if (labelOnly) label = label === "" ? tag : tag + " " + label
    chips.push({
      // The bar turns each chip into a target for its own entry.
      id: entry.id,
      brand: brand,
      icon: brand !== "" ? providerIcon(entry) : (labelOnly ? "" : "󰚩"),
      tag: tag,
      labelOnly: labelOnly,
      label: label,
      // Alert state always follows the highest-percent window, never the
      // pinned one: barWindow changes only the displayed value.
      alarming: isAlarming(entry)
    })
  }
  return chips
}

// The bar presses a slot's widget by geometry: it maps the point into every
// registered click target and takes the newest one whose rect contains it,
// falling back to the slot's button. A chip that registered only its glyph left
// the button to swallow the rest of the slot, so a press aimed at a chip above,
// below or beside it toggled whichever entry was already selected. Each chip
// therefore registers its whole column of the slot, and the gaps between chips
// split at their midpoint: the first and last chips own half of the gap beside
// them, a middle chip a whole one. The outer columns also own the button's
// padding at either end (`edge`), so the columns tile the whole widget and its
// width stays exactly what the plain spacing and padding produced.
function chipHitGaps(index, count, gap, edge) {
  var half = (Number(gap) || 0) / 2
  var outer = Number(edge) || 0
  var chips = Number(count) || 0
  var position = Number(index) || 0
  return {
    left: position === 0 ? outer : half,
    right: position >= chips - 1 ? outer : half
  }
}

// Which usage window the top bar shows. "auto" keeps the historical
// highest-percent metric; the rest pin one window class across vendors.
// Unknown, empty, and legacy values fall back to "auto" so an existing
// shell.json never goes blank after an update.
function normalizeBarWindow(value) {
  var text = cleanText(value, 24).trim().toLowerCase()
  if (text === "" || text === "auto" || text === "highest" || text === "max") return "auto"
  if (text === "session" || text === "5h" || text === "5-hour" || text === "5hour"
      || text === "5hr" || text === "five-hour" || text === "rolling"
      || text === "shortest" || text === "session-5h") return "session"
  if (text === "weekly" || text === "week" || text === "7d" || text === "7-day"
      || text === "weekly-7d") return "weekly"
  if (text === "monthly" || text === "month" || text === "30d" || text === "monthly-cycle") return "monthly"
  return "auto"
}

// The two window lengths the report states exactly. Same values as the Rust
// constants that publish them: opencode_go/vendor.rs ROLLING_WINDOW /
// WEEKLY_WINDOW. A vendor that states any other length falls through to label
// matching instead of being forced into one of these classes.
var SESSION_WINDOW_SECS = 18000
var WEEKLY_WINDOW_SECS = 604800

function metricMatchesWindow(section, want) {
  if (!section || section.type !== "metric") return false
  var label = String(section.label || "")
  var secs = Math.floor(Number(section.window_secs))
  var hasSecs = isFinite(secs) && secs > 0
  var knownSize = hasSecs && (secs === SESSION_WINDOW_SECS || secs === WEEKLY_WINDOW_SECS)
  // A known size is authoritative; an unknown positive size is treated as
  // missing so labels decide. Labels only cover older reports that predate
  // window_secs.
  if (want === "session") {
    if (knownSize) return secs === SESSION_WINDOW_SECS
    return /(\b5\s*-?\s*h(?:ours?|rs?)?\b|\bsessions?\b|\brolling\b)/i.test(label)
  }
  if (want === "weekly") {
    if (knownSize) return secs === WEEKLY_WINDOW_SECS
    return /(\bweek(?:ly|s)?\b|\b7\s*-?\s*d(?:ays?)?\b)/i.test(label)
  }
  if (want === "monthly") {
    // A monthly pool follows the billing cycle and has no single fixed
    // length, so no secs value is authoritative here — except known 5h/7d
    // sizes, which exclude.
    if (knownSize) return false
    return /(\bmonthly\b|\bmonth(?:ly|s)?\b|\bspend\s*\(mo\)|\b30\s*-?\s*d(?:ays?)?\b)/i.test(label)
  }
  return false
}

function maxPercent(sections) {
  var best = null
  for (var i = 0; i < sections.length; i++) {
    var section = sections[i]
    if (section.type === "metric" && (!best || section.percent > best.percent)) best = section
  }
  return best
}

function selectMetric(entry, barWindow) {
  var sections = entry && Array.isArray(entry.sections) ? entry.sections : []
  var metrics = []
  for (var i = 0; i < sections.length; i++) {
    var section = sections[i]
    if (!section || section.type !== "metric") continue
    metrics.push(section)
  }
  if (metrics.length === 0) return null
  var want = normalizeBarWindow(barWindow)
  if (want === "auto") return maxPercent(metrics)
  var candidates = []
  for (var j = 0; j < metrics.length; j++) {
    if (metricMatchesWindow(metrics[j], want)) candidates.push(metrics[j])
  }
  // A pinned window that a vendor does not offer (a balance-only provider,
  // a weekly-only response, no monthly pool) falls back to the historical
  // highest-percent value rather than blanking the bar.
  if (candidates.length === 0) return maxPercent(metrics)
  return maxPercent(candidates)
}

// One Dark defaults used when colors.toml is missing.
var ONE_DARK_PALETTE = {
  green: "#98c379",
  yellow: "#e5c07b",
  orange: "#d19a66",
  red: "#e06c75"
}

// Parse Omarchy theme/colors.toml keys used for RAG.
// Named keys win over the older color1–3 aliases (red.or(color1), not
// last-occurrence-wins). orange falls back to the resolved red when absent.
function parseThemePalette(raw) {
  var greenNamed = ""
  var greenAlias = ""
  var yellowNamed = ""
  var yellowAlias = ""
  var orangeNamed = ""
  var redNamed = ""
  var redAlias = ""
  var lines = String(raw || "").split("\n")
  for (var i = 0; i < lines.length; i++) {
    var match = lines[i].match(/^\s*([A-Za-z0-9_-]+)\s*=\s*["']?(#[0-9A-Fa-f]{6})/)
    if (!match) continue
    var key = match[1]
    var hex = match[2]
    if (key === "green") greenNamed = hex
    else if (key === "color2") greenAlias = hex
    else if (key === "yellow") yellowNamed = hex
    else if (key === "color3") yellowAlias = hex
    else if (key === "orange") orangeNamed = hex
    else if (key === "red") redNamed = hex
    else if (key === "color1") redAlias = hex
  }
  var red = redNamed || redAlias
  var green = greenNamed || greenAlias
  var yellow = yellowNamed || yellowAlias
  var orange = orangeNamed || red
  return {
    green: green || ONE_DARK_PALETTE.green,
    yellow: yellow || ONE_DARK_PALETTE.yellow,
    orange: orange || ONE_DARK_PALETTE.orange,
    red: red || ONE_DARK_PALETTE.red
  }
}

function severityColor(severity, palette) {
  var p = palette || ONE_DARK_PALETTE
  if (severity === "critical") return p.red
  if (severity === "high") return p.orange
  if (severity === "mid") return p.yellow
  if (severity === "low") return p.green
  return p.green
}

/**
 * Lists the metrics of one entry as the settings page offers them, each with whether it is shown and
 * whether its switch may be flipped. Every provider offers one row per metric.
 * @param {Object} entry Entry from the report.
 * @param {*} hidden Raw `hiddenMetrics` setting.
 * @returns {Array<{key: string, label: string, group: string, checked: boolean, canToggle: boolean}>}
 */
function metricChoices(entry, hidden) {
  var rows = []
  if (!entry) return rows
  var off = hiddenKeysFor(hidden, entry.id)
  var sections = Array.isArray(entry.sections) ? entry.sections : []
  var keys = metricKeys(entry)
  for (var i = 0; i < sections.length; i++) {
    if (!sections[i] || sections[i].type !== "metric") continue
    var named = keys[i] || { key: cleanText(sections[i].label, 160).trim(), group: "" }
    rows.push({
      key: named.key,
      label: sections[i].label,
      group: named.group,
      checked: off.indexOf(named.key) < 0,
      canToggle: true
    })
  }
  var shown = rows.filter(function(row) { return row.checked }).length
  return rows.map(function(row) {
    row.canToggle = !row.checked || shown > 1
    return row
  })
}

/**
 * Computes the next `hiddenMetrics` after flipping one row the settings page offers.
 * @param {*} hidden Raw `hiddenMetrics` setting.
 * @param {Object} entry Entry from the report.
 * @param {string} key Row key from `metricChoices`.
 * @returns {Object<string, string[]>}
 */
function toggleMetricChoice(hidden, entry, key) {
  return toggleHiddenMetric(hidden, entry ? entry.id : "", key)
}

/**
 * Decides what the value beside a panel meter reads. A report value that carries information of its own
 * (a dollar balance, a count) is kept. A value that only echoes the percentage is replaced by the percentage
 * in the chosen reading, so the number beside the meter agrees with the bar drawn under it.
 * @param {Object} row Metric row from the report.
 * @param {*} showAs Raw reading setting.
 * @returns {{text: string, left: boolean, percent: number|null}} `left` is true when `percent` is the remainder
 *   and the caller should word it as "left".
 */
function metricValueView(row, showAs) {
  var value = row && row.value !== undefined && row.value !== null ? String(row.value).trim() : ""
  var reading = normalizeShowAs(showAs)
  var left = reading === "left"
  var echoesPercent = /^\d+(\.\d+)?%$/.test(value)
  if (value !== "" && !(left && echoesPercent)) return { text: value, left: false, percent: null }
  var shown = row ? shownPercent(row.percent, reading) : null
  if (shown === null) return { text: "", left: false, percent: null }
  return {
    text: shownPctText(shown) + "%",
    left: left,
    percent: Math.round(shown * 10) / 10
  }
}

function headline(entry, barWindow, showAs) {
  if (!entry) return { text: "", percent: null, severity: "low", label: "" }
  var best = selectMetric(entry, barWindow)
  if (best) {
    // The metric names which of its two numbers goes on the bar; the other one
    // stays in the detail. Reading that beats guessing from the label, which
    // put OpenRouter's dollar figure on the bar and hid its consumed percent.
    // An older report omits the field, and a metric is a percentage by default.
    var bestText = best.headline === "value" && best.value !== ""
      ? best.value : percentText(best.percent, showAs)
    return {
      text: bestText,
      percent: best.percent,
      severity: best.severity,
      label: best.label
    }
  }
  var sections = entry.sections || []
  for (var j = 0; j < sections.length; j++) {
    var row = sections[j]
    if (row.type === "text" && /(balance|available|spend|prepaid)/i.test(row.label) && row.value !== "")
      return { text: row.value, percent: null, severity: "low", label: row.label }
  }
  return { text: entry.status === "error" ? "Error" : "Ready", percent: null, severity: "low", label: "" }
}

function isAlarming(entry) {
  if (!entry) return false
  var summary = headline(entry)
  // Cached and failed fetches stay visible without turning the bar red on their own.
  return summary.severity === "critical"
}

function formatDuration(milliseconds) {
  if (!(milliseconds > 0)) return "now"
  var minutes = Math.floor(milliseconds / 60000)
  var hours = Math.floor(minutes / 60)
  var days = Math.floor(hours / 24)
  if (days > 0) return days + "d " + (hours % 24) + "h"
  if (hours > 0) return hours + "h " + (minutes % 60) + "m"
  return Math.max(1, minutes) + "m"
}

var MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                   "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]

function pad2(value) {
  return ("0" + value).slice(-2)
}

function isSameLocalDay(a, b) {
  return a.getFullYear() === b.getFullYear()
    && a.getMonth() === b.getMonth()
    && a.getDate() === b.getDate()
}

function formatReset(resetAt, nowMs) {
  if (!resetAt) return ""
  var resetMs = new Date(String(resetAt)).getTime()
  if (!isFinite(resetMs)) return ""
  var remaining = resetMs - Number(nowMs)
  if (remaining <= 0) return "Reset due"
  // Show the real local clock time the window reopens, so the absolute
  // reset moment is visible next to the countdown. Date it whenever it
  // lands on another day: a bare "03:00" on an 18h countdown reads as a
  // time that has already passed, which is the ambiguity this row exists
  // to remove. Keyed on the calendar day rather than on "is it 24h away",
  // because tonight's reset crosses midnight long before it crosses 24h.
  var at = new Date(resetMs)
  var clock = pad2(at.getHours()) + ":" + pad2(at.getMinutes())
  if (!isSameLocalDay(at, new Date(Number(nowMs))))
    clock = MONTH_NAMES[at.getMonth()] + " " + at.getDate() + " " + clock
  return "Resets in " + formatDuration(remaining) + " · " + clock
}

function formatUpdated(fetchedAt, nowMs) {
  if (!fetchedAt) return "Updated time unavailable"
  var fetchedMs = new Date(String(fetchedAt)).getTime()
  if (!isFinite(fetchedMs)) return "Updated time unavailable"
  var elapsed = Math.max(0, Number(nowMs) - fetchedMs)
  if (elapsed < 60000) return "Updated just now"
  return "Updated " + formatDuration(elapsed) + " ago"
}

function metricDetail(row) {
  var detail = cleanText(row && row.detail, 1000)
  if (!row || !row.reset_at) return detail
  // Older human-readable reset text remains useful to CLI consumers. Strip
  // just that fragment in the native panel, which renders a live countdown.
  detail = detail.replace(/^Resets in [^·]+\s*(?:·\s*)?/i, "")
  detail = detail.replace(/^Regenerates [^·]+? in [^·]+\s*(?:·\s*)?/i, "")
  detail = detail.replace(/\s*·\s*reset\s+[^·]+$/i, "")
  return detail.trim()
}

function errorMessage(value) {
  var message = cleanText(value, 500).trim()
  return message === "" ? "The usage command failed without an error message." : message
}

// The panel launches ai-usagebar through /usr/bin/env, so a missing binary
// comes back as exit 127 instead of the process simply never starting.
// Quickshell does not emit `exited` when it cannot launch a binary directly --
// it only logs an internal warning -- which used to leave the widget stuck on
// its loading state with no way to explain that the binary was not installed.
function launchErrorMessage(exitCode, stderrText) {
  if (Number(exitCode) === 127)
    return "ai-usagebar is not installed. The plugin is only the display frontend. Install the binary with: omarchy pkg aur add ai-usagebar-bin"
  return errorMessage(stderrText)
}

function settingsId(value) {
  var id = cleanText(value, 80).trim()
  if (!/^[a-z0-9_-]+$/.test(id)
      || id === "__proto__" || id === "constructor" || id === "prototype") return ""
  return id
}

// The Rust bridge deliberately returns only key-presence booleans. Keep this
// parser strict so a compromised/older helper cannot smuggle rich text or an
// unbounded model into the long-lived shell process.
function parseSettingsSnapshot(raw) {
  try {
    var parsed = JSON.parse(String(raw || ""))
    if (!parsed || Number(parsed.schema_version) !== 1
        || !Array.isArray(parsed.primary_choices) || !Array.isArray(parsed.keys))
      return { ok: false, error: "The settings command returned an unsupported response.", primary: "", primary_choices: [], keys: [], vendors: [] }

    var choices = []
    for (var i = 0; i < parsed.primary_choices.length && i < 64; i++) {
      var choice = parsed.primary_choices[i]
      var choiceId = settingsId(choice && choice.id)
      if (choiceId === "") continue
      choices.push({ id: choiceId, value: choiceId, label: cleanText(choice.label, 120) || choiceId })
    }

    var keys = []
    for (var j = 0; j < parsed.keys.length && j < 32; j++) {
      var key = parsed.keys[j]
      var keyId = settingsId(key && key.id)
      if (keyId === "") continue
      keys.push({
        id: keyId,
        label: cleanText(key.label, 120) || keyId,
        environment: cleanText(key.environment, 160),
        secret_label: cleanText(key.secret_label, 120),
        note: cleanText(key.note, 240),
        configured: key.configured === true,
        inline_configured: key.inline_configured === true,
        environment_configured: key.environment_configured === true
      })
    }

    // Provider on/off switches (#244). An older binary sends no vendors list;
    // the section simply stays hidden rather than failing the whole form.
    var vendors = []
    if (Array.isArray(parsed.vendors)) {
      for (var v = 0; v < parsed.vendors.length && v < 64; v++) {
        var vendor = parsed.vendors[v]
        var vendorId = settingsId(vendor && vendor.id)
        if (vendorId === "") continue
        vendors.push({
          id: vendorId,
          label: cleanText(vendor.label, 120) || vendorId,
          enabled: vendor.enabled === true
        })
      }
    }

    var primary = settingsId(parsed.primary)
    var primaryAvailable = false
    for (var k = 0; k < choices.length; k++) {
      if (choices[k].id === primary) {
        primaryAvailable = true
        break
      }
    }
    if (!primaryAvailable) primary = choices.length > 0 ? choices[0].id : ""

    // Quota-threshold notifications ([notifications]). Both values must be
    // well formed or the section stays hidden: a made-up default would show a
    // threshold the bar is not using. An older binary sends neither.
    var notifyThreshold = parsed.notify_threshold
    var notify = null
    if ((parsed.notify_enabled === true || parsed.notify_enabled === false)
        && typeof notifyThreshold === "number" && Number.isInteger(notifyThreshold)
        && notifyThreshold >= 1 && notifyThreshold <= 100)
      notify = { enabled: parsed.notify_enabled, threshold: notifyThreshold }

    return { ok: true, error: "", primary: primary, primary_choices: choices, keys: keys, vendors: vendors, notify: notify }
  } catch (error) {
    return { ok: false, error: "The settings command returned invalid JSON.", primary: "", primary_choices: [], keys: [], vendors: [], notify: null }
  }
}

// Build the stdin patch for `settings apply`. `vendorToggles` (#244) is an
// optional array of {id, enabled}; it is included in the payload only when a
// toggle is pending, so a display-only save keeps the older patch shape a
// pre-#244 binary still accepts. `notifications` follows the same rule: an
// optional {enabled, threshold} that is sent only when the user changed it.
function buildSettingsPatch(primary, changes, vendorToggles, monthlyCycleDay, notifications) {
  var primaryId = settingsId(primary)
  var rawPrimary = String(primary || "").trim()
  if (rawPrimary !== "" && primaryId === "")
    return { ok: false, error: "Choose a valid primary provider.", payload: "" }
  var keys = {}
  var list = Array.isArray(changes) ? changes : []
  var seen = []
  for (var i = 0; i < list.length; i++) {
    var change = list[i] || {}
    var id = settingsId(change.id)
    if (id === "" || seen.indexOf(id) >= 0)
      return { ok: false, error: "A settings row has an invalid provider id.", payload: "" }
    seen.push(id)
    if (change.action === "clear") {
      keys[id] = { action: "clear" }
    } else if (change.action === "set") {
      var value = String(change.value || "")
      if (value === "") return { ok: false, error: "An edited credential is empty.", payload: "" }
      if (value.length > 16384) return { ok: false, error: "A credential is too long.", payload: "" }
      keys[id] = { action: "set", value: value }
    } else return { ok: false, error: "A settings row has an invalid action.", payload: "" }
  }
  var vendors = {}
  var toggles = Array.isArray(vendorToggles) ? vendorToggles : []
  var seenVendors = []
  for (var t = 0; t < toggles.length; t++) {
    var toggle = toggles[t] || {}
    var vendorId = settingsId(toggle.id)
    if (vendorId === "" || seenVendors.indexOf(vendorId) >= 0)
      return { ok: false, error: "A provider switch has an invalid provider id.", payload: "" }
    if (toggle.enabled !== true && toggle.enabled !== false)
      return { ok: false, error: "A provider switch needs an on or off state.", payload: "" }
    seenVendors.push(vendorId)
    vendors[vendorId] = toggle.enabled
  }
  if (monthlyCycleDay !== undefined) {
    if (monthlyCycleDay !== null
        && (typeof monthlyCycleDay !== "number" || !Number.isInteger(monthlyCycleDay)
            || monthlyCycleDay < 1 || monthlyCycleDay > 28))
      return { ok: false, error: "Monthly cycle day must be null or an integer between 1 and 28.", payload: "" }
  }
  if (notifications !== undefined) {
    if (!notifications
        || (notifications.enabled !== true && notifications.enabled !== false)
        || typeof notifications.threshold !== "number" || !Number.isInteger(notifications.threshold)
        || notifications.threshold < 1 || notifications.threshold > 100)
      return { ok: false, error: "The notification threshold must be a whole number between 1 and 100.", payload: "" }
  }
  if (primaryId === "" && seen.length === 0 && seenVendors.length === 0
      && monthlyCycleDay === undefined && notifications === undefined)
    return { ok: false, error: "There are no settings changes to save.", payload: "" }
  var patch = { schema_version: 1, keys: keys }
  if (primaryId !== "") patch.primary = primaryId
  if (seenVendors.length > 0) patch.vendors = vendors
  if (monthlyCycleDay !== undefined) patch.monthly_cycle_day = monthlyCycleDay
  if (notifications !== undefined) {
    patch.notify_enabled = notifications.enabled
    patch.notify_threshold = notifications.threshold
  }
  return {
    ok: true,
    error: "",
    payload: JSON.stringify(patch)
  }
}

function parseSettingsApplyResult(raw) {
  try {
    var parsed = JSON.parse(String(raw || ""))
    return parsed && parsed.ok === true
  } catch (error) {
    return false
  }
}

// Display-only entry choices. Keep exact ids so named accounts are independent.
function barVisibleEntries(entries, hidden) {
  var ids = Array.isArray(hidden) ? hidden : []
  return (Array.isArray(entries) ? entries : []).filter(function(entry) {
    return ids.indexOf(entry.id) < 0
  })
}

function toggleBarEntry(hidden, id) {
  var ids = Array.isArray(hidden) ? hidden : []
  return ids.indexOf(id) >= 0
    ? ids.filter(function(value) { return value !== id })
    : ids.concat([id])
}

// Wheel and middle-click step through the entries the bar is showing.
// `selectedId` is the panel selection. When that entry was hidden, the bar
// is showing the first entry of `entries`, so the step starts there.
function adjacentBarEntryId(entries, selectedId, delta) {
  var list = Array.isArray(entries) ? entries : []
  var step = Number(delta) || 0
  if (list.length === 0 || step === 0) return ""
  var current = -1
  for (var i = 0; i < list.length; i++) {
    if (list[i] && list[i].id === selectedId) {
      current = i
      break
    }
  }
  if (current < 0) current = 0
  var next = (current + step) % list.length
  if (next < 0) next += list.length
  var entry = list[next]
  return entry && entry.id !== undefined && entry.id !== null && entry.id !== ""
    ? String(entry.id) : ""
}

function providerDisplaySummary(rows) {
  var on = 0
  for (var i = 0; i < rows.length; i++) if (rows[i].checked) on++
  return on + "/" + rows.length
}
