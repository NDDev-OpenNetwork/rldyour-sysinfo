import Foundation

@main struct ProtocolTests {
    static func main() throws {
        precondition(MetricFormat.percent(nil) == "—")
        precondition(MetricFormat.percent(.infinity) == "—")
        precondition(MetricFormat.percent(1e100) == "—")
        precondition(MetricFormat.percent(25.4) == "25%")
        precondition(MetricFormat.temperature(80) == "80°")
        precondition(MetricFormat.rate(2048) == "2.0K")
        precondition(MetricFormat.rate(-1) == "—")
        let json = #"{"v":1,"cpu":{"usage":null,"temp":null},"memory":{"used":50,"swap":null},"gpu":{"usage":null,"memory":null,"temp":null},"disk":{"read":0,"write":0,"temp":null},"net":{"rx":1024,"tx":0},"compatible":true}"#
        let sample = try JSONDecoder().decode(MetricSample.self, from: Data(json.utf8))
        precondition(sample.v == 1 && sample.cpu.usage == nil && sample.net.rx == 1024)
        print("PASS: typed v1 samples, unavailable metrics, finite formatting, binary rates")
    }
}
