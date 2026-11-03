import Darwin
import Dispatch
import Foundation
import SEMIControl
import SEMIInterApp
import SEMIMCP

@main
struct StoryProductionProviderMain {
  static func main() async {
    do {
      let configuration = try ProviderConfiguration.parse(Array(CommandLine.arguments.dropFirst()))
      guard let bundleIdentifier = Bundle.main.bundleIdentifier,
        bundleIdentifier == "com.axient.storyproduction.semi-provider"
      else { throw ProviderHostError.invalidBundleIdentity }

      let provider = StoryProvider(configuration: configuration)
      let connection = StoryProviderConnection(
        bundleIdentifier: bundleIdentifier,
        displayName: configuration.displayName,
        instanceID: configuration.instanceID,
        provider: provider)
      let signals = TerminationSignals().stream()
      let worker = Task { await connection.runUntilStopped() }
      for await _ in signals { break }
      worker.cancel()
      await worker.value
    } catch {
      FileHandle.standardError.write(
        Data("story-semi-provider: \(error.localizedDescription)\n".utf8))
      Darwin.exit(2)
    }
  }
}

private struct StoryProviderConnection {
  let bundleIdentifier: String
  let displayName: String
  let instanceID: UUID
  let provider: StoryProvider

  func runUntilStopped() async {
    var failures = 0
    while !Task.isCancelled {
      let server: MCPToolServer
      do {
        let registration = AppRegistration(
          identity: AppIdentity(
            bundleIdentifier: bundleIdentifier,
            displayName: displayName,
            instanceID: instanceID),
          capabilities: StoryProvider.capabilities)
        server = try MCPToolServer(registration: registration, provider: provider)
      } catch {
        log("provider setup failed")
        break
      }

      do {
        let client = try await SEMIInterAppClient.connect(server: server)
        failures = 0
        log("connected to SEMI")
        await waitForDisconnect(client)
        let peerCleanupConfirmed = await client.disconnectAndWait()
        if !peerCleanupConfirmed {
          log("SEMI peer cleanup was not confirmed; the local provider endpoint is closed")
        }
      } catch is CancellationError {
        await server.close()
        break
      } catch {
        await server.close()
        guard !Task.isCancelled else { break }
        log("connection unavailable")
        failures = min(failures + 1, 6)
        let delay = min(30, 1 << (failures - 1))
        do { try await Task.sleep(for: .seconds(Int64(delay))) } catch { break }
      }
    }
  }

  private func waitForDisconnect(_ client: SEMIInterAppClient) async {
    for await status in client.updates() {
      if Task.isCancelled { return }
      switch status {
      case .connecting, .connected:
        continue
      case .interrupted, .invalidated, .disconnected:
        return
      }
    }
  }

  private func log(_ message: String) {
    FileHandle.standardError.write(Data("story-semi-provider: \(message)\n".utf8))
  }
}

private final class TerminationSignals: @unchecked Sendable {
  private let lock = NSLock()
  private var sources: [DispatchSourceSignal] = []
  private var finished = false

  func stream() -> AsyncStream<Void> {
    signal(SIGINT, SIG_IGN)
    signal(SIGTERM, SIG_IGN)
    return AsyncStream { continuation in
      let values = [SIGINT, SIGTERM].map { value in
        let source = DispatchSource.makeSignalSource(signal: value, queue: .global())
        source.setEventHandler { [weak self] in
          guard self?.finish() == true else { return }
          continuation.yield(())
          continuation.finish()
        }
        return source
      }
      lock.lock()
      sources = values
      lock.unlock()
      continuation.onTermination = { [weak self] _ in _ = self?.finish() }
      for source in values { source.resume() }
    }
  }

  private func finish() -> Bool {
    lock.lock()
    guard !finished else {
      lock.unlock()
      return false
    }
    finished = true
    let active = sources
    sources = []
    lock.unlock()
    for source in active { source.cancel() }
    return true
  }
}

private enum ProviderHostError: Error, LocalizedError {
  case invalidBundleIdentity
  var errorDescription: String? { "The provider executable has an unexpected bundle identity." }
}
