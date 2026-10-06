import AppKit
import Foundation

/// Seconds between samples requested from the daemon — the same default the
/// GNOME extension's schema ships. `defaults write
/// com.nddev-opennetwork.rldyour-sysinfo interval N` overrides it; 0 selects
/// the daemon's realtime mode.
private let requestedInterval =
    min(max(UserDefaults.standard.object(forKey: "interval") as? Int ?? 5, 0), 60)
/// Shown wherever the host cannot supply a metric.
private let absent = MetricFormat.absent

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let menu = NSMenu()
    private var rows: [String: NSMenuItem] = [:]
    private var client: MetricsClient?

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem.button?.font = NSFont.monospacedDigitSystemFont(ofSize: 12, weight: .medium)
        statusItem.button?.title = "SYS …"
        for title in ["Processor", "Processor temperature", "Memory", "Swap", "Graphics", "Graphics memory", "Graphics temperature", "Disk read", "Disk write", "Disk temperature", "Network in", "Network out"] {
            let item = NSMenuItem(title: "\(title): \(absent)", action: nil, keyEquivalent: "")
            rows[title] = item
            menu.addItem(item)
        }
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit rldyour sysinfo", action: #selector(quitApp), keyEquivalent: "q")
        quit.target = self
        menu.addItem(quit)
        statusItem.menu = menu
        client = MetricsClient(interval: requestedInterval) { [weak self] event in
            Task { @MainActor in
                switch event {
                case .sample(let sample): self?.apply(sample)
                case .offline: self?.offline()
                }
            }
        }
        client?.start()
    }

    func applicationWillTerminate(_ notification: Notification) { client?.stop(); client = nil }

    @objc private func quitApp() { NSApplication.shared.terminate(nil) }

    private func offline() {
        if statusItem.button?.title != "SYS offline" { statusItem.button?.title = "SYS offline" }
        for title in rows.keys { set(title, absent) }
    }

    private func apply(_ sample: MetricSample) {
        let cpu = sample.cpu
        let memory = sample.memory
        let gpu = sample.gpu
        let disk = sample.disk
        let net = sample.net
        let title = "CPU \(MetricFormat.percent(cpu.usage))  RAM \(MetricFormat.percent(memory.used))  GPU \(MetricFormat.percent(gpu.usage))  ↓\(MetricFormat.rate(net.rx)) ↑\(MetricFormat.rate(net.tx))"
        if statusItem.button?.title != title { statusItem.button?.title = title }
        set("Processor", percent(cpu.usage))
        set("Processor temperature", temperature(cpu.temp))
        set("Memory", percent(memory.used))
        set("Swap", percent(memory.swap))
        set("Graphics", percent(gpu.usage))
        set("Graphics memory", percent(gpu.memory))
        set("Graphics temperature", temperature(gpu.temp))
        set("Disk read", rate(disk.read))
        set("Disk write", rate(disk.write))
        set("Disk temperature", temperature(disk.temp))
        set("Network in", rate(net.rx))
        set("Network out", rate(net.tx))
    }

    private func set(_ row: String, _ value: String) {
        let title = "\(row): \(value)"
        if rows[row]?.title != title { rows[row]?.title = title }
    }
}

private func percent(_ value: Double?) -> String { MetricFormat.percent(value) }
private func temperature(_ value: Double?) -> String { MetricFormat.temperature(value) }
private func rate(_ value: Double?) -> String { MetricFormat.rate(value) }

@main
struct SysinfoApp {
    @MainActor static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.setActivationPolicy(.accessory)
        app.run()
    }
}
