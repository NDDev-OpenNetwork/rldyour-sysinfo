import Foundation

struct MetricSample: Decodable, Sendable {
    struct CPU: Decodable, Sendable { let usage: Double?; let temp: Double? }
    struct Memory: Decodable, Sendable { let used: Double?; let swap: Double? }
    struct GPU: Decodable, Sendable { let usage: Double?; let memory: Double?; let temp: Double? }
    struct Disk: Decodable, Sendable { let read: Double?; let write: Double?; let temp: Double? }
    struct Network: Decodable, Sendable { let rx: Double?; let tx: Double? }
    let v: Int
    let cpu: CPU
    let memory: Memory
    let gpu: GPU
    let disk: Disk
    let net: Network
}

enum MetricFormat {
    static let absent = "—"
    static func percent(_ value: Double?) -> String {
        guard let value, value.isFinite, (0...100).contains(value) else { return absent }
        return String(format: "%.0f%%", value)
    }
    static func temperature(_ value: Double?) -> String {
        guard let value, value.isFinite, (-100...200).contains(value) else { return absent }
        return String(format: "%.0f°", value)
    }
    static func rate(_ value: Double?) -> String {
        guard var amount = value, amount.isFinite, amount >= 0 else { return absent }
        let units = ["B", "K", "M", "G", "T"]
        var unit = 0
        while amount >= 1024 && unit < units.count - 1 { amount /= 1024; unit += 1 }
        let digits = unit > 0 && amount < 10 ? 1 : 0
        return "\(String(format: "%.*f", digits, amount))\(units[unit])"
    }
}
