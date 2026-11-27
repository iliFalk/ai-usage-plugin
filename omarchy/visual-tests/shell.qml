import QtQuick
import QtQuick.Window
import Quickshell
import qs.Commons as Commons
import "Usage"

// Deterministic, offscreen visual scenarios for the real Quattro settings
// component. scripts/omarchy-visual-test.sh supplies the shell modules and
// output directory, then launches this config with Qt's offscreen backend.
ShellRoot {
  id: root

  readonly property string outputDir: Quickshell.env("AI_USAGEBAR_VISUAL_OUTPUT_DIR")
  property int scenario: 0
  property color scenarioBackground: "#1e1e2e"

  function applyTheme(foreground, background, accent, urgent, muted) {
    // FileView may finish loading the user's current theme after startup.
    // Reapply the fixture immediately before each capture so output is stable
    // and the test never depends on or mutates personal theme files.
    Commons.Color.themeShellValues = ({})
    Commons.Color.userShellValues = ({})
    Commons.Color.mergeShell()
    Commons.Color.foreground = foreground
    Commons.Color.background = background
    Commons.Color.accent = accent
    Commons.Color.urgent = urgent
    Commons.Color.muted = muted
    Commons.Style.cornerRadius = 8
  }

  function focusFirst(item) {
    if (!item) return false
    if (item.activeFocusOnTab === true && item.visible && item.enabled) {
      item.forceActiveFocus()
      return true
    }
    var children = item.children || []
    for (var i = 0; i < children.length; i++) {
      if (focusFirst(children[i])) return true
    }
    return false
  }

  function prepareScenario() {
    if (scenario === 0) {
      applyTheme("#cdd6f4", "#1e1e2e", "#89b4fa", "#f38ba8", "#6c7086")
      scenarioBackground = "#1e1e2e"
      settings.foreground = "#cdd6f4"
      settings.urgent = "#f38ba8"
      settings.openSection = "display"
      viewport.contentY = 0
    } else if (scenario === 2) {
      // Quota-threshold notifications ([notifications]): the section is only
      // drawn when the snapshot carried the values, so the scenario supplies
      // them the same way it supplies the rest of the fake settings.
      applyTheme("#cdd6f4", "#1e1e2e", "#89b4fa", "#f38ba8", "#6c7086")
      scenarioBackground = "#1e1e2e"
      settings.foreground = "#cdd6f4"
      settings.urgent = "#f38ba8"
      settings.notifySettings = ({ enabled: true, threshold: 97 })
      settings.openSection = "notifications"
      viewport.contentY = 0
    } else {
      applyTheme("#1f2328", "#f6f8fa", "#0969da", "#cf222e", "#656d76")
      scenarioBackground = "#f6f8fa"
      settings.foreground = "#1f2328"
      settings.urgent = "#cf222e"
      settings.openSection = "credentials"
      viewport.contentY = 0
    }
    settings.forceActiveFocus()
    Qt.callLater(function() { root.focusFirst(settings) })
    captureDelay.restart()
  }

  function capture() {
    var name = scenario === 0
      ? "settings-display-dark.png"
      : scenario === 2
        ? "settings-notifications-dark.png"
        : "settings-credentials-light.png"
    window.contentItem.grabToImage(function(result) {
      if (!result.saveToFile(outputDir + "/" + name)) {
        console.error("VISUAL SCENARIO FAIL: could not write " + name)
        Qt.exit(1)
        return
      }
      console.log("VISUAL SCENARIO OK: " + name)
      scenario++
      if (scenario < 3) prepareScenario()
      else Qt.quit()
    }, Qt.size(window.width, window.height))
  }

  Window {
    id: window
    width: 560
    height: 760
    visible: true
    title: "AI Usage visual scenarios"

    Rectangle {
      anchors.fill: parent
      color: root.scenarioBackground
    }

    Flickable {
      id: viewport
      anchors.fill: parent
      anchors.margins: 24
      clip: true
      contentWidth: width
      contentHeight: settings.implicitHeight

      SettingsView {
        id: settings
        width: viewport.width
        fontFamily: "monospace"
        showValue: true
        showProvider: true
        showAll: false
        colorCodeUsage: true
        brandIcons: true
        selectedPrimary: "opencode-go"
        barProviderChoices: [
          { id: "opencode-go", name: "OpenCode Go", checked: true },
          { id: "openrouter", name: "OpenRouter", checked: true },
          { id: "deepseek", name: "DeepSeek", checked: false }
        ]
        metricEntries: [
          { id: "opencode-go", name: "OpenCode Go", rows: [
            { key: "Rolling (5h)", label: "Rolling (5h)", group: "", checked: true, canToggle: true },
            { key: "Weekly (7d)", label: "Weekly (7d)", group: "", checked: true, canToggle: true },
            { key: "Monthly", label: "Monthly", group: "", checked: true, canToggle: true }
          ] }
        ]
        snapshot: ({
          primary_choices: [
            { id: "opencode-go", value: "opencode-go", label: "OpenCode Go" },
            { id: "openrouter", value: "openrouter", label: "OpenRouter" },
            { id: "deepseek", value: "deepseek", label: "DeepSeek" }
          ],
          vendors: [
            { id: "opencode-go", label: "OpenCode Go", enabled: true },
            { id: "openrouter", label: "OpenRouter", enabled: true },
            { id: "deepseek", label: "DeepSeek", enabled: false }
          ],
          keys: [
            { id: "opencode-go", label: "OpenCode Go", environment: "OPENCODE_GO_API_KEY",
              secret_label: "API key", note: "usage quota",
              configured: true, inline_configured: true, environment_configured: false },
            { id: "deepseek", label: "DeepSeek", environment: "DEEPSEEK_API_KEY",
              secret_label: "API key", note: "",
              configured: false, inline_configured: false, environment_configured: false }
          ]
        })
        notifySettings: ({ enabled: true, threshold: 97 })
      }
    }
  }

  Timer {
    id: startup
    interval: 150
    running: true
    repeat: false
    onTriggered: root.prepareScenario()
  }

  Timer {
    id: captureDelay
    interval: 180
    repeat: false
    onTriggered: root.capture()
  }

  Timer {
    interval: 10000
    running: true
    repeat: false
    onTriggered: {
      console.error("VISUAL SCENARIO FAIL: timed out")
      Qt.exit(1)
    }
  }
}
