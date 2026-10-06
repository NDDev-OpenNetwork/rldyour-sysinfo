import Foundation

/// All mutable state belongs to the serial queue. The Sendable callback carries
/// value-type samples only; AppKit is never accessed on this queue.
final class MetricsClient: @unchecked Sendable {
    enum Event: Sendable { case sample(MetricSample); case offline }
    private let queue = DispatchQueue(label: "com.nddev-opennetwork.rldyour-sysinfo.socket", qos: .utility)
    private let onEvent: @Sendable (Event) -> Void
    private let interval: Int
    private let path: String
    private var reader: DispatchSourceRead?
    private var retry: DispatchWorkItem?
    private var buffer = Data()
    private let decoder = JSONDecoder()
    private var stopped = false
    private static let maxLine = 4096

    init(interval: Int, path: String = FileManager.default.homeDirectoryForCurrentUser
         .appendingPathComponent("Library/Application Support/rldyour-sysinfo/rldyour-sysinfo.sock").path,
         onEvent: @escaping @Sendable (Event) -> Void) {
        self.interval = min(max(interval, 0), 60)
        self.path = path
        self.onEvent = onEvent
    }

    func start() { queue.async { self.connect() } }
    func stop() {
        queue.async {
            self.stopped = true
            self.retry?.cancel()
            self.retry = nil
            self.close()
        }
    }

    private func connect() {
        guard !stopped, reader == nil else { return }
        retry = nil
        let fd = Darwin.socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { return reconnect() }
        var ownsFD = true
        defer { if ownsFD { Darwin.close(fd) } }
        var noSigPipe: Int32 = 1
        guard setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &noSigPipe, socklen_t(MemoryLayout<Int32>.size)) == 0,
              fcntl(fd, F_SETFD, FD_CLOEXEC) == 0 else { return reconnect() }

        var address = sockaddr_un()
        address.sun_family = sa_family_t(AF_UNIX)
        guard path.utf8.count < MemoryLayout.size(ofValue: address.sun_path) else { return reconnect() }
        address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        withUnsafeMutableBytes(of: &address.sun_path) { raw in
            raw.initializeMemory(as: UInt8.self, repeating: 0)
            for (index, byte) in path.utf8.enumerated() { raw[index] = byte }
        }
        let connected = withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        guard connected == 0 else { return reconnect() }
        let announcement = Data("{\"interval\":\(interval)}\n".utf8)
        let written = announcement.withUnsafeBytes { Darwin.write(fd, $0.baseAddress, $0.count) }
        guard written == announcement.count,
              fcntl(fd, F_SETFL, fcntl(fd, F_GETFL) | O_NONBLOCK) == 0 else { return reconnect() }

        let source = DispatchSource.makeReadSource(fileDescriptor: fd, queue: queue)
        source.setEventHandler { [weak self] in self?.read(fd) }
        source.setCancelHandler { Darwin.close(fd) }
        reader = source
        ownsFD = false
        source.resume()
    }

    private func read(_ fd: Int32) {
        var bytes = [UInt8](repeating: 0, count: Self.maxLine)
        while true {
            let count = Darwin.read(fd, &bytes, bytes.count)
            if count < 0 {
                if errno == EINTR { continue }
                if errno == EAGAIN || errno == EWOULDBLOCK { return }
                return reconnect()
            }
            if count == 0 { return reconnect() }
            buffer.append(contentsOf: bytes.prefix(count))
            while let newline = buffer.firstIndex(of: 10) {
                guard buffer.distance(from: buffer.startIndex, to: newline) <= Self.maxLine else { return reconnect() }
                let line = buffer.prefix(upTo: newline)
                guard let sample = try? decoder.decode(MetricSample.self, from: line), sample.v == 1 else {
                    return reconnect()
                }
                buffer.removeSubrange(...newline)
                onEvent(.sample(sample))
            }
            if buffer.count > Self.maxLine { return reconnect() }
        }
    }

    private func close() {
        reader?.cancel()
        reader = nil
        buffer.removeAll(keepingCapacity: true)
    }

    private func reconnect() {
        close()
        guard !stopped else { return }
        onEvent(.offline)
        retry?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.connect() }
        retry = work
        queue.asyncAfter(deadline: .now() + 5, execute: work)
    }
}
