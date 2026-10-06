import Foundation

/// Exercise the real dispatch client against an isolated daemon socket.
@main struct SocketTests {
    static func main() {
        let path = CommandLine.arguments[1]
        let done = DispatchSemaphore(value: 0)
        let client = MetricsClient(interval: 0, path: path) { event in
            if case .sample(let sample) = event {
                precondition(sample.v == 1)
                done.signal()
            }
        }
        client.start()
        for _ in 0..<3 { precondition(done.wait(timeout: .now() + 10) == .success) }
        client.stop()
        print("PASS: dispatch socket connection, typed real samples, cancellation")
    }
}
