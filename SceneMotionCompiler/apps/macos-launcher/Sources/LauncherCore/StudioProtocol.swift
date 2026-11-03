import Foundation

public struct StudioReady: Decodable, Equatable, Sendable {
    public let origin: String
    public let projectRoot: String
}

public struct StudioFailure: Decodable, Equatable, Sendable {
    public let code: String
    public let message: String
}

public enum StudioProtocolEvent: Equatable, Sendable {
    case ready(StudioReady)
    case failure(StudioFailure)
    case ignored
}

public func parseStudioProtocolLine(_ line: String) -> StudioProtocolEvent {
    let readyPrefix = "SMC_STUDIO_READY\t"
    let failurePrefix = "SMC_STUDIO_ERROR\t"
    if line.hasPrefix(readyPrefix),
       let bytes = String(line.dropFirst(readyPrefix.count)).data(using: .utf8),
       let ready = try? JSONDecoder().decode(StudioReady.self, from: bytes) {
        return .ready(ready)
    }
    if line.hasPrefix(failurePrefix),
       let bytes = String(line.dropFirst(failurePrefix.count)).data(using: .utf8),
       let failure = try? JSONDecoder().decode(StudioFailure.self, from: bytes) {
        return .failure(failure)
    }
    return .ignored
}
