import Foundation
import Subprocess

struct StoryCLI: Sendable {
  let configuration: ProviderConfiguration

  func read(operation: String, inputJSON: String) async throws -> String {
    let result = try await invoke(operation: operation, inputJSON: inputJSON)
    guard result.succeeded, let output = result.standardOutput
    else { throw StoryCLIError.commandFailed }
    return output
  }

  func execute(operation: String, inputJSON: String) async throws -> StoryCLIResult {
    try await invoke(operation: operation, inputJSON: inputJSON)
  }

  private func invoke(operation: String, inputJSON: String) async throws -> StoryCLIResult {
    try Task.checkCancellation()
    try configuration.validateWorkspace()
    let arguments = [
      "--story-root", configuration.storyRoot.path,
      "text",
      "--workspace", configuration.workspaceDescriptorPath,
      "--geultu-bin", configuration.geultu.path,
      "author", operation, "--input", "-",
    ]
    guard inputJSON.utf8.count <= 65_536 else { throw StoryCLIError.inputTooLarge }
    let process = Task { try await invoke(arguments: arguments, inputJSON: inputJSON) }

    return try await withTaskCancellationHandler {
      try await withThrowingTaskGroup(of: StoryCLIResult.self) { group in
        group.addTask { try await process.value }
        group.addTask {
          try await Task.sleep(for: .seconds(120))
          throw StoryCLIError.timeout
        }
        do {
          guard let output = try await group.next() else { throw StoryCLIError.emptyResult }
          group.cancelAll()
          return output
        } catch {
          process.cancel()
          group.cancelAll()
          throw error
        }
      }
    } onCancel: {
      process.cancel()
    }
  }

  private func invoke(arguments: [String], inputJSON: String) async throws -> StoryCLIResult {
    var platformOptions = PlatformOptions()
    platformOptions.processGroupID = 0
    platformOptions.teardownSequence = [
      .gracefulShutDown(toProcessGroup: true, allowedDurationToNextStep: .seconds(1))
    ]
    platformOptions.preSpawnProcessConfigurator = { _, fileActions in
      try Task.checkCancellation()
      try configuration.addWorkspaceWorkingDirectoryAction(to: &fileActions)
    }
    let environment: [Environment.Key: String?] = [
      Environment.Key(stringLiteral: "STORY_SEMI_WORKSPACE_FD"): .some(
        String(configuration.workspaceChildDescriptor))
    ]
    let result = try await Subprocess.run(
      .path(.init(configuration.storyctl.path)),
      arguments: Arguments(arguments),
      environment: .inherit.updating(environment),
      platformOptions: platformOptions,
      input: .string(inputJSON + "\n"),
      output: .string(limit: 900 * 1_024),
      error: .string(limit: 64 * 1_024)
    )
    do {
      try configuration.validateWorkspace()
    } catch {
      throw ProviderConfigurationError.workspaceChangedDuringOwnerCall
    }

    let succeeded: Bool
    if case .exited(0) = result.terminationStatus {
      succeeded = true
    } else {
      succeeded = false
    }
    return StoryCLIResult(
      succeeded: succeeded,
      standardOutput: result.standardOutput,
      standardError: result.standardError
    )
  }
}

struct StoryCLIResult: Sendable {
  let succeeded: Bool
  let standardOutput: String?
  let standardError: String?
}

private enum StoryCLIError: Error, LocalizedError {
  case commandFailed
  case emptyResult
  case inputTooLarge
  case timeout

  var errorDescription: String? {
    switch self {
    case .commandFailed: "StoryProduction returned a failed or malformed read result."
    case .emptyResult: "StoryProduction did not return a result."
    case .inputTooLarge: "The StoryProduction request exceeds the local provider limit."
    case .timeout: "StoryProduction read exceeded its time limit."
    }
  }
}
