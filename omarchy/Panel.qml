import QtQuick
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Commons as Commons
import qs.Ui
import "Model.js" as Model
import "I18n.js" as I18n

// Native Omarchy Quattro popup. BarWidget.qml owns the bar slot and injects
// its button as this panel's anchor; collection stays in the Rust binary.
Panel {
  id: root
  moduleName: "akitaonrails.ai-usagebar"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null
  readonly property var barIdentity: hostWidget || root

  readonly property color foreground: bar ? bar.foreground : Commons.Color.foreground
  readonly property color urgent: bar ? bar.urgent : Commons.Color.urgent
  readonly property color dim: Qt.darker(foreground, 1.45)
  readonly property color track: Style.selectedFillFor(foreground, Commons.Color.accent)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  readonly property bool vertical: bar ? bar.vertical : false

  // RAG palette from Omarchy colors.toml.
  property string hexGreen: "#98c379"
  property string hexYellow: "#e5c07b"
  property string hexOrange: "#d19a66"
  property string hexRed: "#e06c75"
  property color themeGreen: hexGreen
  property color themeYellow: hexYellow
  property color themeOrange: hexOrange
  property color themeRed: hexRed

  property var entries: []
  property string primaryProvider: ""
  property string selectedEntryId: ""
  property string loadError: ""
  property string commandStderr: ""
  property string commandStdout: ""
  property bool loading: true
  property int lastExitCode: 0
  property bool refreshQueued: false
  property double lastSuccessfulMs: 0
  property double nowMs: Date.now()
  property bool keyboardActive: false
  property bool settingsOpen: false

  readonly property int refreshIntervalSec: Math.max(30, Math.min(3600,
    Number(setting("refreshIntervalSec", 300)) || 300))
  readonly property string configuredProvider: String(setting("provider", "") || "").trim()
  readonly property string rememberedEntryId: String(setting("lastSelectedEntryId", "") || "").trim()
  readonly property bool showValue: Model.booleanSetting(setting("showValue", true), true)
  readonly property bool showProvider: Model.booleanSetting(setting("showProvider", false), false)
  readonly property bool showAll: Model.booleanSetting(setting("showAll", false), false)
  readonly property bool colorCodeUsage: Model.booleanSetting(setting("colorCodeUsage", false), false)
  readonly property string uiLocaleSetting: String(setting("uiLocale", "auto") || "auto")
  // Bind Qt.locale so Auto tracks the shell language without a restart.
  readonly property string uiLocale: I18n.resolveLocale(uiLocaleSetting, Qt.locale().name)
  readonly property string barWindow: Model.normalizeBarWindow(setting("barWindow", "auto"))
  readonly property bool brandIcons: Model.booleanSetting(setting("brandIcons", true), true)
  readonly property string showAs: Model.normalizeShowAs(setting("showAs", "used"))
  readonly property var hiddenMetrics: Model.normalizeHiddenMetrics(setting("hiddenMetrics", {}))
  readonly property var visibleEntries: Model.filteredEntries(entries, configuredProvider)
  readonly property int entryIndex: Model.selectedIndex(visibleEntries, selectedEntryId)
  readonly property var entry: entryIndex >= 0 ? visibleEntries[entryIndex] : null
  readonly property string entryFetchedAt: {
    if (!entry) return ""
    return String(entry.fetched_at || "")
  }
  readonly property var shapedEntries: visibleEntries.map(function(item) {
    return Model.visibleEntry(item, Model.hiddenKeysFor(hiddenMetrics, item.id))
  })
  readonly property var hiddenBarEntries: setting("hiddenBarEntries", [])
  readonly property var barEntries: Model.barVisibleEntries(shapedEntries, hiddenBarEntries)
  readonly property var barEntry: {
    for (var i = 0; i < barEntries.length; i++)
      if (barEntries[i].id === selectedEntryId) return barEntries[i]
    return barEntries.length > 0 ? barEntries[0] : null
  }
  readonly property var barProviderChoices: visibleEntries.map(function(item) {
    return { id: item.id, name: Model.providerName(item),
      checked: !Array.isArray(hiddenBarEntries) || hiddenBarEntries.indexOf(item.id) < 0 }
  })
  readonly property var shapedEntry: entryIndex >= 0 ? shapedEntries[entryIndex] : null
  readonly property var summary: Model.headline(shapedEntry, barWindow, showAs)
  // barWindow pins the bar value and its echoes (hero detail, tooltip).
  // Panel rows and alert state keep the historical highest-percent headline.
  readonly property var entrySections: entry
    ? Model.visibleEntry(entry, Model.hiddenKeysFor(hiddenMetrics, entry.id)).sections : []
  readonly property var metricEntries: visibleEntries.map(function(item) {
    return {
      id: item.id,
      name: Model.providerName(item),
      rows: Model.metricChoices(item, hiddenMetrics)
    }
  }).filter(function(item) { return item.rows.length > 0 })
  readonly property bool filterMiss: configuredProvider !== "" && entries.length > 0 && visibleEntries.length === 0
  readonly property bool entryAlarming: shownAlarming()
  // A cached or failed provider response stays a status line, but a report
  // that never arrived has nothing else to show on the bar.
  readonly property bool reportMissing: loadError !== "" && entries.length === 0
  readonly property bool alarming: (showAll ? shownAnyAlarming() : entryIsAlarming(barEntry))
    || reportMissing

  function tr(key, params) {
    return I18n.t(uiLocale, key, params || null)
  }

  function setUiLocale(value) {
    var next = I18n.normalizeLocaleTag(value)
    if (next === "") next = "auto"
    if (next === uiLocaleSetting) return
    persistWidgetSettings({ uiLocale: next })
  }

  function alpha(color, opacity) {
    return Qt.rgba(color.r, color.g, color.b, opacity)
  }

  function clamp(value, low, high) {
    return Math.max(low, Math.min(high, value))
  }

  function applyThemePalette(raw) {
    var parsed = Model.parseThemePalette(raw)
    hexGreen = parsed.green
    hexYellow = parsed.yellow
    hexOrange = parsed.orange
    hexRed = parsed.red
    themeGreen = hexGreen
    themeYellow = hexYellow
    themeOrange = hexOrange
    themeRed = hexRed
  }

  // critical prefers Quattro urgent (theme-linked); other rungs use colors.toml.
  // When colorCodeUsage is off, everything falls back to the bar foreground.
  function severityColorOf(severity) {
    if (!colorCodeUsage) return foreground
    if (severity === "critical") return urgent
    if (severity === "high") return themeOrange
    if (severity === "mid") return themeYellow
    if (severity === "low") return themeGreen
    return foreground
  }

  function severityHexOf(severity) {
    if (!colorCodeUsage) return ""
    if (severity === "critical") return hexRed
    if (severity === "high") return hexOrange
    if (severity === "mid") return hexYellow
    if (severity === "low") return hexGreen
    return ""
  }

  FileView {
    id: themeColorsFile
    path: Commons.Color.currentThemePath + "/colors.toml"
    watchChanges: true
    printErrors: false
    onLoaded: root.applyThemePalette(text())
    onFileChanged: reload()
    onLoadFailed: root.applyThemePalette("")
  }

  Connections {
    target: Commons.Color
    function onUrgentChanged() { themeColorsFile.reload() }
    function onAccentChanged() { themeColorsFile.reload() }
    function onForegroundChanged() { themeColorsFile.reload() }
  }

  function syncSelection() {
    if (visibleEntries.length === 0) {
      selectedEntryId = ""
      return
    }
    // The persisted choice is the source of truth. A refresh gap (fetch
    // error, sleep/wake stale list) briefly drops entries; the fallback
    // below then re-resolves to the primary and that transient selection
    // used to stick — the chosen entry came back and was ignored until a
    // shell restart. Once the remembered entry is back in the list, it wins
    // over any selection that only exists because of that gap.
    var remembered = rememberedEntryId
    if (remembered !== "") {
      for (var r = 0; r < visibleEntries.length; r++)
        if (visibleEntries[r].id === remembered) {
          selectedEntryId = remembered
          return
        }
    }
    for (var i = 0; i < visibleEntries.length; i++)
      if (visibleEntries[i].id === selectedEntryId) return
    selectedEntryId = Model.preferredEntryId(visibleEntries, primaryProvider, rememberedEntryId)
  }

  function restoreRememberedSelection() {
    if (visibleEntries.length === 0) return
    selectedEntryId = Model.preferredEntryId(visibleEntries, primaryProvider, rememberedEntryId)
  }

  function persistWidgetSettings(values) {
    // Quattro persists inline widget settings in shell.json and pushes them
    // live to every monitor. Keep every existing setting, including settings
    // introduced by future versions, and apply only the requested overrides.
    var entry = Model.settingsWithOverrides(root.settings, root.moduleName, values)
    if (!entry) return false

    // Apply locally first so controls remain responsive. Older compatible hosts
    // without updateEntryInline still retain the choice for this session.
    root.settings = entry
    if (hostWidget && "settings" in hostWidget) hostWidget.settings = entry
    if (bar && bar.shell && typeof bar.shell.updateEntryInline === "function")
      bar.shell.updateEntryInline(root.moduleName, entry)
    return true
  }

  function persistSelection(entryId) {
    if (String(entryId || "").trim() === rememberedEntryId) return
    persistWidgetSettings({ lastSelectedEntryId: entryId })
  }

  function setShowValue(enabled) {
    var next = enabled === true
    if (next === showValue) return
    persistWidgetSettings({ showValue: next })
  }

  function setShowProvider(enabled) {
    var next = enabled === true
    if (next === showProvider) return
    persistWidgetSettings({ showProvider: next })
  }

  function toggleBarProvider(entryId) {
    persistWidgetSettings({ hiddenBarEntries: Model.toggleBarEntry(hiddenBarEntries, entryId) })
  }

  function setShowAll(enabled) {
    var next = enabled === true
    if (next === showAll) return
    persistWidgetSettings({ showAll: next })
  }

  function setColorCodeUsage(enabled) {
    var next = enabled === true
    if (next === colorCodeUsage) return
    persistWidgetSettings({ colorCodeUsage: next })
  }

  function setBarWindow(value) {
    var next = Model.normalizeBarWindow(value)
    if (next === barWindow) return
    persistWidgetSettings({ barWindow: next })
  }

  function setBrandIcons(enabled) {
    var next = enabled === true
    if (next === brandIcons) return
    persistWidgetSettings({ brandIcons: next })
  }

  function setShowAs(value) {
    var next = Model.normalizeShowAs(value)
    if (next === showAs) return
    persistWidgetSettings({ showAs: next })
  }

  function setMetricShown(entryId, key) {
    var target = null
    for (var i = 0; i < visibleEntries.length; i++)
      if (visibleEntries[i].id === entryId) target = visibleEntries[i]
    if (!target) return
    if (!Model.canToggleMetric(target, hiddenMetrics, key)) return
    persistWidgetSettings({ hiddenMetrics: Model.toggleMetricChoice(hiddenMetrics, target, key) })
  }

  function entryIsAlarming(item) {
    return Model.isAlarming(item)
  }

  function shownAlarming() {
    return entryIsAlarming(shapedEntry)
  }

  function shownAnyAlarming() {
    for (var i = 0; i < barEntries.length; i++)
      if (entryIsAlarming(barEntries[i])) return true
    return false
  }

  function selectEntry(index) {
    if (visibleEntries.length === 0) return
    var wrapped = ((index % visibleEntries.length) + visibleEntries.length) % visibleEntries.length
    selectedEntryId = visibleEntries[wrapped].id
    persistSelection(selectedEntryId)
    if (providerList.visible) providerList.forceLayout()
    if (panelFlick) panelFlick.contentY = 0
  }

  // Wheel and middle-click belong to the bar, so they step through the
  // entries the bar is showing. The panel's left/right keys still walk every
  // tab, including a provider hidden from the bar. With nothing left on the
  // bar, the gesture keeps walking the panel.
  function selectAdjacentBarEntry(delta) {
    if (!delta || visibleEntries.length === 0) return
    if (barEntries.length === 0) {
      selectEntry(entryIndex + delta)
      return
    }
    var id = Model.adjacentBarEntryId(barEntries, selectedEntryId, delta)
    for (var i = 0; i < visibleEntries.length; i++) {
      if (visibleEntries[i].id === id) {
        selectEntry(i)
        return
      }
    }
  }

  // A bar chip stands for one entry when the bar shows several: select it,
  // then open the panel on it. The chip the panel already shows toggles
  // closed, the way the bar button does.
  function openEntry(entryId) {
    var wanted = String(entryId || "")
    for (var i = 0; i < visibleEntries.length; i++) {
      if (visibleEntries[i].id !== wanted) continue
      if (opened && i === entryIndex) {
        close()
        return
      }
      selectEntry(i)
      open()
      return
    }
    open()
  }

  function startRefresh() {
    if (usageProcess.running) {
      refreshQueued = true
      return
    }
    refreshQueued = false
    commandStdout = ""
    commandStderr = ""
    if (entries.length === 0) loading = true
    usageProcess.running = true
  }

  function finishRefresh() {
    var parsed = Model.parseReport(commandStdout)
    if (parsed.ok) {
      primaryProvider = parsed.primary
      entries = parsed.entries
      loadError = ""
      lastSuccessfulMs = Date.now()
      syncSelection()
    } else {
      var detail = commandStderr.trim()
      loadError = lastExitCode === 127
        ? Model.launchErrorMessage(lastExitCode, detail)
        : (detail !== "" ? Model.errorMessage(detail) : parsed.error)
    }
    loading = false
    if (refreshQueued) Qt.callLater(startRefresh)
  }

  function refresh() { startRefresh() }

  function openSettings() {
    settingsOpen = true
    keyboardActive = false
    if (panelFlick) panelFlick.contentY = 0
    Qt.callLater(function() { settingsView.forceActiveFocus() })
  }

  function closeSettings() {
    settingsOpen = false
    if (panelFlick) panelFlick.contentY = 0
    Qt.callLater(function() { keyCatcher.forceActiveFocus() })
  }

  function switchPanel(direction) {
    if (bar && typeof bar.switchPanelFrom === "function")
      return bar.switchPanelFrom(barIdentity, direction)
    return false
  }

  function statusMessage() {
    if (filterMiss) return root.tr("status.filter_miss", { id: configuredProvider })
    if (entry && entry.error !== "") return entry.error
    if (loadError !== "") return entries.length > 0
      ? root.tr("status.refresh_failed", { error: loadError })
      : loadError
    if (entry && entry.stale) return root.tr("status.cached")
    return ""
  }

  function statusIsUrgent() {
    return filterMiss || (entry && entry.error !== "") || loadError !== ""
  }

  function heroMeta() {
    if (!entry) return loading ? root.tr("hero.loading") : root.tr("hero.usage_report")
    if (entry.error !== "") return root.tr("hero.provider_unavailable")
    var text = entry.plan || root.tr("hero.usage_limits")
    if (entry.stale) text += " · " + root.tr("tip.cached")
    return Model.autoTextSafe(text)
  }

  function shownEntries() {
    if (showAll) return barEntries
    return barEntry ? [barEntry] : []
  }

  readonly property var barChips: labeledChips()

  function labeledChips() {
    var chips = Model.barChips(
      barEntries, barEntry, showAll, showValue, showProvider, loading, alarming, vertical,
      barWindow, showAs, brandIcons)
    if (!showValue || vertical) return chips
    var rows = shownEntries()
    var next = []
    for (var i = 0; i < chips.length; i++) {
      var chip = chips[i]
      if (chip.label === "!" || i >= rows.length) {
        next.push(chip)
        continue
      }
      // The headline severity drives icon RAG when colour-coding is on.
      var head = Model.headline(rows[i], barWindow, showAs)
      next.push({
        id: chip.id,
        brand: chip.brand,
        icon: chip.icon,
        labelOnly: chip.labelOnly,
        label: chip.label,
        alarming: chip.alarming,
        severity: head && head.severity ? head.severity : ""
      })
    }
    return next
  }

  function barText() {
    if (showAll)
      return Model.barStrip(barEntries, alarming, vertical, showValue, showProvider, loading,
        barWindow, showAs)
    var value = barEntry ? Model.headline(barEntry, barWindow, showAs).text : ""
    return Model.barLabel(alarming, vertical, showValue, loading,
      barEntry !== null, value, showProvider ? Model.providerShort(barEntry) : "",
      Model.providerIcon(barEntry))
  }

  function tooltipLines(item) {
    var raw = String(Model.headline(item, barWindow, showAs).text || "").split("\n")
    var lines = []
    for (var i = 0; i < raw.length; i++) {
      var line = Model.autoTextSafe(raw[i]).trim()
      if (line !== "") lines.push(line)
    }
    return lines
  }

  // One coloured hover row per line, tinted by the headline severity.
  function tooltipRows() {
    function rowsFor(item) {
      var lines = tooltipLines(item)
      if (lines.length === 0) return []
      var head = Model.headline(item, barWindow, showAs)
      var sev = head && head.severity ? head.severity : "low"
      if (lines.length > 1) {
        var multi = []
        for (var i = 0; i < lines.length; i++) {
          var bit = lines[i]
          if (item && item.stale && i === lines.length - 1) bit += " · " + root.tr("tip.cached")
          multi.push({ text: bit, severity: sev })
        }
        return multi
      }
      var single = Model.providerName(item)
      if (lines.length === 1) single += " · " + lines[0]
      if (item && item.stale) single += " · " + root.tr("tip.cached")
      return [{ text: single, severity: sev }]
    }

    if (showAll && barEntries.length > 0) {
      var chips = []
      for (var e = 0; e < barEntries.length; e++) {
        var chunk = rowsFor(barEntries[e])
        for (var c = 0; c < chunk.length; c++) chips.push(chunk[c])
      }
      return chips
    }
    if (!entry) {
      var msg = Model.autoTextSafe(statusMessage() || root.tr("app.name"))
      return msg !== "" ? [{ text: msg, severity: "" }] : []
    }
    return barEntry ? rowsFor(barEntry) : [{ text: root.tr("providers.none_shown"), severity: "" }]
  }

  readonly property var ragTooltipRows: tooltipRows()
  readonly property string plainTooltipText: {
    var rows = ragTooltipRows
    if (rows.length > 0) {
      var lines = []
      for (var i = 0; i < rows.length; i++) {
        var text = String((rows[i] && rows[i].text) || "").trim()
        if (text !== "") lines.push(text)
      }
      if (lines.length > 0) return lines.join("\n")
    }
    return Model.autoTextSafe(statusMessage() || root.tr("app.name"))
  }

  onEntriesChanged: Qt.callLater(syncSelection)
  onConfiguredProviderChanged: Qt.callLater(syncSelection)
  onRememberedEntryIdChanged: Qt.callLater(restoreRememberedSelection)
  onOpenedChanged: {
    if (opened) {
      keyboardActive = false
      nowMs = Date.now()
      if (panelFlick) panelFlick.contentY = 0
      if (lastSuccessfulMs === 0 || nowMs - lastSuccessfulMs >= refreshIntervalSec * 1000)
        startRefresh()
      Qt.callLater(function() { keyCatcher.forceActiveFocus() })
    } else {
      settingsOpen = false
    }
  }

  Timer {
    interval: root.refreshIntervalSec * 1000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: root.startRefresh()
  }

  Timer {
    interval: 30000
    running: root.opened
    repeat: true
    onTriggered: root.nowMs = Date.now()
  }

  Process {
    id: usageProcess
    running: false
    // /usr/bin/env always starts on Omarchy and reports a missing ai-usagebar
    // as exit 127. Keep the command as structured argv: no shell is needed.
    command: ["/usr/bin/env", "ai-usagebar", "usage", "--json"]

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        root.commandStdout = text
      }
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: root.commandStderr = text
    }

    onExited: function(exitCode) {
      root.lastExitCode = exitCode
      // Let both waitForEnd collectors publish their buffers first.
      Qt.callLater(function() { root.finishRefresh() })
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.barIdentity
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(390))
    contentHeight: panel.fittedContentHeight(column.implicitHeight, Style.space(640))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      // Native form controls own Tab/Enter/Esc while settings are open.
      blocked: root.settingsOpen

      onMoveRequested: function(dx, dy) {
        if (!root.settingsOpen && dx !== 0) {
          root.keyboardActive = true
          root.selectEntry(root.entryIndex + dx)
        }
        if (dy !== 0)
          panelFlick.contentY = root.clamp(panelFlick.contentY + dy * Style.space(96), 0,
            Math.max(0, panelFlick.contentHeight - panelFlick.height))
      }
      onActivateRequested: if (!root.settingsOpen) root.refresh()
      onCloseRequested: root.settingsOpen ? root.closeSettings() : root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(text) {
        if (!root.settingsOpen && (text === "r" || text === "R")) root.refresh()
        else if (!root.settingsOpen && (text === "s" || text === "S")) root.openSettings()
      }

      Flickable {
        id: panelFlick
        anchors.fill: parent
        contentWidth: width
        contentHeight: column.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        WheelHandler {
          target: null
          enabled: panelFlick.interactive
          acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
          onWheel: function(event) {
            if (event.pixelDelta.y === 0 && event.angleDelta.y === 0) {
              event.accepted = false
              return
            }
            // Keep touchpad motion smooth while covering more of the long settings form.
            var delta = event.pixelDelta.y !== 0
              ? event.pixelDelta.y * 5
              : event.angleDelta.y / 120 * Style.space(144)
            panelFlick.contentY = root.clamp(panelFlick.contentY - delta,
              0, Math.max(0, panelFlick.contentHeight - panelFlick.height))
          }
        }

        Column {
          id: column
          // The provider tabs are bordered buttons, and the first one in each
          // row sits flush against this Flickable's clip edge. At fractional
          // device scales (a 1.25 monitor scale, the shell font at its 12px
          // base) Qt snaps the 1px border to a device pixel that the clip
          // discards, so that tab renders with three borders. Keep a hairline
          // of slack on both sides so no control sits exactly on the clip
          // boundary. (#231)
          x: Style.spacing.hairline
          width: panelFlick.width - Style.spacing.hairline * 2
          spacing: Style.space(12)

          PanelHero {
            width: parent.width
            title: root.settingsOpen ? root.tr("hero.settings")
              : (root.entry ? Model.providerName(root.entry) : root.tr("app.name"))
            meta: root.settingsOpen ? root.tr("hero.settings_meta") : root.heroMeta()
            // Long settings copy must not live in the hero detail pill — that
            // control is a single-line chip and overflows in RU/pt-BR. The
            // sentence renders as a wrapped caption under the hero instead.
            detail: root.settingsOpen ? ""
              : (root.entry && root.summary.text !== "Ready" && root.summary.text !== root.tr("ready")
                ? Model.autoTextSafe(root.summary.text) : "")
            foreground: root.foreground
            fontFamily: root.fontFamily

            iconComponent: Component {
              BrandMark {
                brand: root.settingsOpen || !root.brandIcons ? "" : Model.brandIconFile(root.entry)
                fallback: root.settingsOpen ? "󰒓"
                  : (root.brandIcons ? Model.providerIcon(root.entry) : "󰚩")
                // Colour-coding: full RAG. Off: binary foreground vs urgent
                // when the summary is critical.
                foreground: root.colorCodeUsage
                  ? root.severityColorOf(root.summary.severity || "")
                  : ((root.summary.severity || "") === "critical"
                    ? root.urgent : root.foreground)
                fontFamily: root.fontFamily
                fontSize: Style.font.display
              }
            }

            trailingControl: Component {
              Row {
                spacing: Style.space(4)

                PanelActionButton {
                  visible: !root.settingsOpen
                  iconText: "󰑐"
                  tooltipText: root.tr("action.refresh")
                  foreground: root.foreground
                  fontFamily: root.fontFamily
                  enabled: !usageProcess.running
                  onClicked: root.refresh()
                }

                PanelActionButton {
                  iconText: root.settingsOpen ? "󰁍" : "󰒓"
                  tooltipText: root.settingsOpen ? root.tr("action.back") : root.tr("action.settings")
                  foreground: root.foreground
                  fontFamily: root.fontFamily
                  onClicked: root.settingsOpen ? root.closeSettings() : root.openSettings()
                }
              }
            }
          }

          Text {
            visible: root.settingsOpen
            width: parent.width
            text: root.tr("hero.settings_detail")
            textFormat: Text.PlainText
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            wrapMode: Text.WordWrap
          }

          SettingsView {
            id: settingsView
            visible: root.settingsOpen
            width: parent.width
            foreground: root.foreground
            urgent: root.urgent
            fontFamily: root.fontFamily
            showValue: root.showValue
            showProvider: root.showProvider
            showAll: root.showAll
            colorCodeUsage: root.colorCodeUsage
            uiLocale: root.uiLocale
            uiLocaleSetting: root.uiLocaleSetting
            barWindow: root.barWindow
            showAs: root.showAs
            brandIcons: root.brandIcons
            barProviderChoices: root.barProviderChoices
            onBarProviderToggleRequested: function(entryId) { root.toggleBarProvider(entryId) }
            metricEntries: root.metricEntries
            onSaved: root.startRefresh()
            onShowValueRequested: function(enabled) { root.setShowValue(enabled) }
            onShowProviderRequested: function(enabled) { root.setShowProvider(enabled) }
            onShowAllRequested: function(enabled) { root.setShowAll(enabled) }
            onColorCodeUsageRequested: function(enabled) { root.setColorCodeUsage(enabled) }
            onUiLocaleRequested: function(value) { root.setUiLocale(value) }
            onBarWindowRequested: function(value) { root.setBarWindow(value) }
            onShowAsRequested: function(value) { root.setShowAs(value) }
            onMetricToggleRequested: function(entryId, key) { root.setMetricShown(entryId, key) }
            onBrandIconsRequested: function(enabled) { root.setBrandIcons(enabled) }
            onCloseRequested: root.closeSettings()
          }

          // Providers wrap into additional rows instead of being clipped by
          // the panel edge once there are more configured entries than fit
          // on one line — a fixed-width ListView silently hid entries past
          // the visible edge, with no way to reach them (see #173).
          Flow {
            id: providerList
            visible: !root.settingsOpen && root.visibleEntries.length > 1
            width: parent.width
            height: visible ? implicitHeight : 0
            flow: Flow.LeftToRight
            spacing: Style.spacing.md

            Repeater {
              model: root.visibleEntries

              delegate: Button {
                required property var modelData
                required property int index

                height: Style.spacing.controlHeight
                width: implicitWidth
                text: Model.providerName(modelData)
                selected: index === root.entryIndex
                hasCursor: root.keyboardActive && index === root.entryIndex
                bordered: true
                foreground: root.foreground
                fontFamily: root.fontFamily
                fontSize: Style.font.bodySmall
                verticalPadding: Style.spacing.controlPaddingY
                onClicked: {
                  root.keyboardActive = true
                  root.selectEntry(index)
                }
                onHovered: function(isHovered) { if (isHovered) root.keyboardActive = true }
              }
            }
          }

          BorderSurface {
            readonly property string message: root.statusMessage()
            visible: !root.settingsOpen && message !== ""
            width: parent.width
            implicitHeight: statusText.implicitHeight + Style.spacing.xl * 2
            color: root.alpha(root.statusIsUrgent() ? root.urgent : root.foreground, 0.09)
            borderSpec: Border.flat(root.alpha(root.statusIsUrgent() ? root.urgent : root.foreground, 0.35), 1)
            radius: Style.cornerRadius

            Text {
              id: statusText
              anchors.left: parent.left
              anchors.right: parent.right
              anchors.verticalCenter: parent.verticalCenter
              anchors.leftMargin: Style.space(12)
              anchors.rightMargin: Style.space(12)
              text: parent.message
              textFormat: Text.PlainText
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              wrapMode: Text.WordWrap
            }
          }

          Column {
            visible: !root.settingsOpen && root.loading && root.entries.length === 0
            width: parent.width
            spacing: Style.space(8)

            PanelSectionHeader {
              text: root.tr("section.usage")
              foreground: root.foreground
              fontFamily: root.fontFamily
            }

            Text {
              width: parent.width
              text: root.tr("loading.providers")
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.body
              horizontalAlignment: Text.AlignHCenter
            }
          }

          Column {
            id: usageSection
            visible: !root.settingsOpen && root.entrySections.length > 0
            width: parent.width
            spacing: Style.space(8)

            PanelSeparator {
              width: parent.width
              foreground: root.foreground
            }

            PanelSectionHeader {
              text: root.tr("section.usage_balance")
              foreground: root.foreground
              fontFamily: root.fontFamily
            }

            Repeater {
              model: root.entrySections

              Column {
                required property var modelData
                width: usageSection.width

                // `visible` alone is not enough: QML evaluates the bindings of
                // hidden items too, so every row used to be handed to all three
                // components and the two that did not match read fields the row
                // does not carry. A "spacer" row has no label or value, which is
                // what produced the TypeError below on every report.
                MetricRow {
                  visible: modelData.type === "metric"
                  width: parent.width
                  row: modelData.type === "metric" ? modelData : null
                }

                DetailRow {
                  visible: modelData.type === "text"
                  width: parent.width
                  row: modelData.type === "text" ? modelData : null
                }

                BlockRow {
                  visible: modelData.type === "block"
                  width: parent.width
                  row: modelData.type === "block" ? modelData : null
                }

                Item {
                  visible: modelData.type === "spacer"
                  width: 1
                  height: Style.space(4)
                }
              }
            }
          }

          GraphicRow {
            width: usageSection.width
            visible: root.entry
              && String(Model.baseProvider(root.entry.id)) === "opencode-go"
              && root.entrySections.length > 0
          }

          Text {
            visible: !root.settingsOpen && !root.loading && !root.entry && root.statusMessage() === ""
            width: parent.width
            topPadding: Style.space(20)
            text: root.tr("empty.no_usage")
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
          }

          Text {
            visible: !root.settingsOpen && root.entryFetchedAt !== ""
            width: parent.width
            topPadding: Style.space(2)
            text: I18n.formatUpdated(root.entryFetchedAt, root.nowMs, root.uiLocale)
              + (usageProcess.running ? " · refreshing…" : "")
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
          }
        }
      }
    }
  }

  component MetricRow: Column {
    id: metricRow
    property var row: null
    readonly property string severity: row ? String(row.severity || "") : ""
    readonly property color ragColor: root.severityColorOf(metricRow.severity)
    readonly property string detailText: I18n.displayDetail(root.uiLocale, Model.metricDetail(row))
    readonly property string resetText: row ? I18n.formatReset(row.reset_at, root.nowMs, root.uiLocale) : ""
    readonly property int shownPercent: row ? Model.shownPercent(row.percent, root.showAs) : 0
    readonly property var valueView: Model.metricValueView(row, root.showAs)
    readonly property string valueText: valueView.left
      ? I18n.t(root.uiLocale, "metric.left", { percent: valueView.percent })
      : valueView.text

    spacing: Style.space(6)

    Item {
      width: parent.width
      implicitHeight: Math.max(metricLabel.implicitHeight, metricValue.implicitHeight)

      Text {
        id: metricLabel
        text: metricRow.row ? metricRow.row.label : ""
        textFormat: Text.PlainText
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        elide: Text.ElideRight
        anchors.left: parent.left
        anchors.right: metricValue.left
        anchors.rightMargin: Style.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
      }

      Text {
        id: metricValue
        text: metricRow.valueText
        textFormat: Text.PlainText
        color: metricRow.ragColor
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        font.bold: true
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
      }
    }

    Item {
      width: parent.width
      implicitHeight: Math.max(Style.space(4), Math.round(Style.spacing.controlHeight * 0.14))

      Rectangle {
        id: meterTrack
        anchors.fill: parent
        radius: height / 2
        color: root.track
      }

      Rectangle {
        anchors.left: meterTrack.left
        anchors.verticalCenter: meterTrack.verticalCenter
        height: meterTrack.height
        radius: meterTrack.radius
        width: meterTrack.width * root.clamp(metricRow.shownPercent / 100, 0, 1)
        color: metricRow.ragColor

        Behavior on width {
          NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
        }
      }
    }

    Text {
      visible: text !== ""
      width: parent.width
      leftPadding: metricRow.indent
      text: metricRow.detailText
      textFormat: Text.PlainText
      color: root.dim
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
      wrapMode: Text.WordWrap
    }

    Text {
      visible: text !== ""
      width: parent.width
      text: metricRow.resetText
      textFormat: Text.PlainText
      color: root.dim
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
      // The row now carries a date and clock as well as the countdown, so it
      // can outgrow a narrow panel. Wrap like the detail line above it rather
      // than painting past the panel edge.
      wrapMode: Text.WordWrap
    }
  }

  component DetailRow: Item {
    id: detailRow
    property var row: null
    readonly property bool heading: row && row.label !== "" && row.value === ""

    implicitHeight: heading ? headingLabel.implicitHeight : Math.max(detailLabel.implicitHeight, detailValue.implicitHeight)

    PanelSectionHeader {
      id: headingLabel
      visible: detailRow.heading
      width: parent.width
      text: detailRow.row ? Model.autoTextSafe(String(detailRow.row.label || "").toUpperCase()) : ""
      foreground: root.foreground
      fontFamily: root.fontFamily
    }

    Text {
      id: detailLabel
      visible: !detailRow.heading && text !== ""
      text: detailRow.row ? detailRow.row.label : ""
      textFormat: Text.PlainText
      color: root.foreground
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      font.bold: true
      anchors.left: parent.left
      anchors.top: parent.top
      width: Math.min(implicitWidth, parent.width * 0.42)
      elide: Text.ElideRight
    }

    Text {
      id: detailValue
      visible: !detailRow.heading
      text: detailRow.row ? detailRow.row.value : ""
      textFormat: Text.PlainText
      color: root.dim
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      horizontalAlignment: detailLabel.visible ? Text.AlignRight : Text.AlignLeft
      wrapMode: Text.WordWrap
      anchors.left: detailLabel.visible ? detailLabel.right : parent.left
      anchors.leftMargin: detailLabel.visible ? Style.spacing.md : 0
      anchors.right: parent.right
      anchors.top: parent.top
    }
  }

  component BlockRow: Column {
    id: blockRow
    property var row: null

    spacing: Style.space(4)

    Text {
      width: parent.width
      text: blockRow.row ? blockRow.row.label : ""
      textFormat: Text.PlainText
      color: root.foreground
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      font.bold: true
      elide: Text.ElideRight
    }

    Text {
      width: parent.width
      text: blockRow.row && blockRow.row.body ? blockRow.row.body.join("\n") : ""
      textFormat: Text.PlainText
      color: root.dim
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
      wrapMode: Text.WordWrap
    }
  }

  component GraphicRow: Column {
    id: graphicRow
    property var row: null
    spacing: Style.space(8)

    readonly property bool forOpenCodeGo: root.entry
      ? String(Model.baseProvider(root.entry.id)) === "opencode-go"
      : false

    Text {
      width: parent.width
      text: root.tr("graphic.title")
      textFormat: Text.PlainText
      color: root.foreground
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      font.bold: true
      visible: graphicRow.forOpenCodeGo
    }

    Canvas {
      id: graphicCanvas
      width: parent.width
      height: graphicRow.forOpenCodeGo ? 150 : 0
      visible: graphicRow.forOpenCodeGo

      function metricByLabel(label) {
        var sections = root.entrySections || []
        for (var i = 0; i < sections.length; i++) {
          var s = sections[i]
          if (s && s.type === "metric" && String(s.label || "") === label)
            return s
        }
        return null
      }

      function elapsedFrac(resetAt, windowSecs) {
        if (!resetAt || !windowSecs) return 0
        var resetMs = new Date(String(resetAt)).getTime()
        var nowMs = Number(root.nowMs)
        if (!isFinite(resetMs)) return 0
        var startMs = resetMs - Number(windowSecs) * 1000
        var elapsed = nowMs - startMs
        return Math.max(0, Math.min(1, elapsed / Number(windowSecs) / 1000))
      }

      function monthlyElapsedFrac(resetAt) {
        // OpenCode Go does not report the monthly cycle length (it follows the
        // subscriber's billing date, 28/29/31 days), so estimate a 30-day
        // cycle purely to place the monthly marker.
        if (!resetAt) return 0
        var resetMs = new Date(String(resetAt)).getTime()
        var nowMs = Number(root.nowMs)
        if (!isFinite(resetMs)) return 0
        var startMs = resetMs - 30 * 86400000
        var elapsed = nowMs - startMs
        return Math.max(0, Math.min(1, elapsed / (30 * 86400000)))
      }

      onPaint: {
        if (!graphicRow.forOpenCodeGo) return
        var ctx = getContext("2d")
        if (!ctx) return
        var w = graphicCanvas.width
        var h = graphicCanvas.height
        var padX = Style.space(6)
        var padY = Style.space(6)
        var plotLeft = padX
        var plotRight = w - padX
        var plotTop = padY
        var plotBottom = h - padY
        var plotW = plotRight - plotLeft
        var plotH = plotBottom - plotTop

        function xFor(frac) { return plotLeft + frac * plotW }
        function yFor(pct) { return plotBottom - Math.min(Math.max(pct, 0), 100) / 100 * plotH }

        ctx.clearRect(0, 0, w, h)

        // Axes
        ctx.strokeStyle = root.dim
        ctx.lineWidth = 1
        ctx.beginPath()
        ctx.moveTo(plotLeft, plotBottom)
        ctx.lineTo(plotRight, plotBottom)
        ctx.moveTo(plotLeft, plotBottom)
        ctx.lineTo(plotLeft, plotTop)
        ctx.stroke()

        // Linear expected usage: on pace from 0 to 100 across the window
        ctx.strokeStyle = root.alpha(root.foreground, 0.35)
        ctx.lineWidth = 1
        ctx.beginPath()
        ctx.moveTo(xFor(0), yFor(0))
        ctx.lineTo(xFor(1), yFor(100))
        ctx.stroke()

        // Weekly usage line
        var weekly = metricByLabel("Weekly (7d)")
        var weeklyPct = weekly ? weekly.percent : 0
        var weeklyFrac = weekly ? elapsedFrac(weekly.reset_at, weekly.window_secs) : 0
        ctx.strokeStyle = root.hexGreen
        ctx.lineWidth = 2
        ctx.beginPath()
        ctx.moveTo(xFor(0), yFor(0))
        ctx.lineTo(xFor(weeklyFrac), yFor(weeklyPct))
        ctx.stroke()
        ctx.fillStyle = root.hexGreen
        ctx.beginPath()
        ctx.arc(xFor(weeklyFrac), yFor(weeklyPct), 3, 0, 2 * Math.PI)
        ctx.fill()

        // Monthly usage line. When the report carries an exact window length
        // (cycle day configured in settings), pace against it; otherwise fall
        // back to a 30-day estimate.
        var monthly = metricByLabel("Monthly")
        var monthlyPct = monthly ? monthly.percent : 0
        var monthlyFrac = monthly
          ? monthly.window_secs
            ? elapsedFrac(monthly.reset_at, monthly.window_secs)
            : monthlyElapsedFrac(monthly.reset_at)
          : 0
        ctx.strokeStyle = root.hexOrange
        ctx.lineWidth = 2
        ctx.beginPath()
        ctx.moveTo(xFor(0), yFor(0))
        ctx.lineTo(xFor(monthlyFrac), yFor(monthlyPct))
        ctx.stroke()
        ctx.fillStyle = root.hexOrange
        ctx.beginPath()
        ctx.arc(xFor(monthlyFrac), yFor(monthlyPct), 3, 0, 2 * Math.PI)
        ctx.fill()
      }
    }
  }
}
