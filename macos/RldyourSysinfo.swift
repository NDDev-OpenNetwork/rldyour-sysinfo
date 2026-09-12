import AppKit
import Foundation

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let menu = NSMenu()
    private var rows: [String: NSMenuItem] = [:]
    private var stopped = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem.button?.font = NSFont.monospacedDigitSystemFont(ofSize: 12, weight: .medium)
        statusItem.button?.title = "SYS …"
        for title in ["Processor", "Processor temperature", "Memory", "Swap", "Graphics", "Graphics memory", "Graphics temperature", "Disk read", "Disk write", "Disk temperature", "Network in", "Network out"] {
            let item = NSMenuItem(title: "\(title): —", action: nil, keyEquivalent: "")
            rows[title] = item
            menu.addItem(item)
        }
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit rldyour sysinfo", action: #selector(quitApp), keyEquivalent: "q")
        quit.target = self
        menu.addItem(quit)
        statusItem.menu = menu
        connect()
    }

    func applicationWillTerminate(_ notification: Notification) { stopped = true }

    @objc private func quitApp() { NSApplication.shared.terminate(nil) }

    private func connect() {
        DispatchQueue.global(qos: .utility).async { [weak self] in
            guard let self, !self.stopped else { return }
            let socket = Foundation.SocketPort()
            _ = socket
            let fd = Darwin.socket(AF_UNIX, SOCK_STREAM, 0)
            guard fd >= 0 else { return self.retry() }
            defer { Darwin.close(fd) }

            var address = sockaddr_un()
            address.sun_family = sa_family_t(AF_UNIX)
            let path = FileManager.default.homeDirectoryForCurrentUser
                .appendingPathComponent("Library/Caches/rldyour-sysinfo/rldyour-sysinfo.sock").path
            guard path.utf8.count < MemoryLayout.size(ofValue: address.sun_path) else { return self.retry() }
            withUnsafeMutableBytes(of: &address.sun_path) { raw in
                raw.initializeMemory(as: UInt8.self, repeating: 0)
                for (index, byte) in path.utf8.enumerated() { raw[index] = byte }
            }
            let connected = withUnsafePointer(to: &address) {
                $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    Darwin.connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }
            guard connected == 0 else { return self.retry() }
            _ = "{\"interval\":2}\n".withCString { Darwin.write(fd, $0, strlen($0)) }

            let handle = FileHandle(fileDescriptor: fd, closeOnDealloc: false)
            var buffer = Data()
            while !self.stopped {
                guard let chunk = try? handle.read(upToCount: 4096), !chunk.isEmpty else { break }
                buffer.append(chunk)
                while let newline = buffer.firstIndex(of: 10) {
                    let line = buffer[..<newline]
                    buffer.removeSubrange(...newline)
                    if let object = try? JSONSerialization.jsonObject(with: line) as? [String: Any] {
                        DispatchQueue.main.async { self.apply(object) }
                    }
                }
            }
            self.retry()
        }
    }

    private func retry() {
        guard !stopped else { return }
        DispatchQueue.main.async { self.statusItem.button?.title = "SYS offline" }
        DispatchQueue.global().asyncAfter(deadline: .now() + 3) { self.connect() }
    }

    private func apply(_ sample: [String: Any]) {
        let cpu = dictionary(sample, "cpu")
        let memory = dictionary(sample, "memory")
        let gpu = dictionary(sample, "gpu")
        let disk = dictionary(sample, "disk")
        let net = dictionary(sample, "net")
        statusItem.button?.title = "CPU \(percent(cpu["usage"]))  RAM \(percent(memory["used"]))  GPU \(percent(gpu["usage"]))  ↓\(rate(net["rx"])) ↑\(rate(net["tx"]))"
        set("Processor", percent(cpu["usage"]))
        set("Processor temperature", temperature(cpu["temp"]))
        set("Memory", percent(memory["used"]))
        set("Swap", percent(memory["swap"]))
        set("Graphics", percent(gpu["usage"]))
        set("Graphics memory", percent(gpu["memory"]))
        set("Graphics temperature", temperature(gpu["temp"]))
        set("Disk read", rate(disk["read"]))
        set("Disk write", rate(disk["write"]))
        set("Disk temperature", temperature(disk["temp"]))
        set("Network in", rate(net["rx"]))
        set("Network out", rate(net["tx"]))
    }

    private func set(_ row: String, _ value: String) { rows[row]?.title = "\(row): \(value)" }
}

private func dictionary(_ value: [String: Any], _ key: String) -> [String: Any] { value[key] as? [String: Any] ?? [:] }
private func number(_ value: Any?) -> Double? { (value as? NSNumber)?.doubleValue }
private func percent(_ value: Any?) -> String { number(value).map { String(format: "%.1f%%", $0) } ?? "—" }
private func temperature(_ value: Any?) -> String { number(value).map { String(format: "%.1f°C", $0) } ?? "—" }
private func rate(_ value: Any?) -> String {
    guard var amount = number(value) else { return "—" }
    let units = ["B/s", "KB/s", "MB/s", "GB/s"]
    var unit = 0
    while amount >= 1000 && unit < units.count - 1 { amount /= 1000; unit += 1 }
    return String(format: amount >= 100 ? "%.0f %@" : "%.1f %@", amount, units[unit])
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
