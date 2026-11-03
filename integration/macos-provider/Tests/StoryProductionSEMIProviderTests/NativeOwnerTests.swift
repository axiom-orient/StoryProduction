import Darwin
import Foundation
import SEMIControl
import SEMICore
import Testing

@testable import StoryProductionSEMIProvider

private struct NativeOwnerFixture {
  let root: URL
  let scratch: URL
  let workspace: URL
  let storyctl: URL
  let geultu: URL
  let provider: StoryProvider

  init() throws {
    var root = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { root.deleteLastPathComponent() }
    self.root = root
    let environment = ProcessInfo.processInfo.environment
    storyctl = URL(
      fileURLWithPath: environment["STORY_PROVIDER_TEST_STORYCTL"]
        ?? root.appending(path: "integration/target/debug/storyctl").path)
    geultu = URL(
      fileURLWithPath: environment["STORY_PROVIDER_TEST_GEULTU"]
        ?? root.appending(path: "geultu/target/release/geultu").path)
    try #require(
      FileManager.default.isExecutableFile(atPath: storyctl.path),
      "Build native storyctl before this integration test.")
    try #require(
      FileManager.default.isExecutableFile(atPath: geultu.path),
      "Build native geultu before this integration test.")
    let temporaryPath = try #require(realpath(FileManager.default.temporaryDirectory.path, nil))
    defer { free(temporaryPath) }
    scratch = URL(fileURLWithPath: String(cString: temporaryPath))
      .appending(path: "story-native-provider-\(UUID().uuidString)")
    workspace = scratch.appending(path: "workspace")
    try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
    let source = scratch.appending(path: "source.md")
    try "A synthetic author preserves a source in a closed archive.".write(
      to: source, atomically: true, encoding: .utf8)
    _ = try Self.run(
      storyctl,
      arguments: [
        "--story-root", root.path, "text", "--workspace", workspace.path, "--geultu-bin",
        geultu.path,
        "create", "--project", "native-provider", "--title", "Synthetic native provider",
        "--source", source.path, "--actor", "synthetic-test",
      ])
    provider = StoryProvider(
      configuration: try ProviderConfiguration.parse([
        "--story-root", root.path, "--workspace", workspace.path,
        "--storyctl-bin", storyctl.path, "--geultu-bin", geultu.path,
        "--instance-id", UUID().uuidString,
      ]))
  }

  func cleanUp() { try? FileManager.default.removeItem(at: scratch) }

  static func run(_ executable: URL, arguments: [String], input: String? = nil) throws -> String {
    let process = Process()
    process.executableURL = executable
    process.arguments = arguments
    var environment = ProcessInfo.processInfo.environment
    environment["PATH"] = "/story-provider-test-no-interpreters"
    process.environment = environment
    let output = Pipe()
    let error = Pipe()
    let stdin = Pipe()
    process.standardOutput = output
    process.standardError = error
    process.standardInput = stdin
    try process.run()
    if let input { try stdin.fileHandleForWriting.write(contentsOf: Data(input.utf8)) }
    try stdin.fileHandleForWriting.close()
    let bytes = output.fileHandleForReading.readDataToEndOfFile()
    let diagnostic = error.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    try #require(
      process.terminationStatus == 0,
      "Native owner call failed: \(String(decoding: diagnostic, as: UTF8.self))")
    return String(decoding: bytes, as: UTF8.self)
  }

  func status() async throws -> [String: Any] {
    let result = try await provider.read(
      capabilityID: "story.authoring.status", inputJSON: "{}", operationID: UUID())
    try #require(result.outcome == .succeeded)
    return try StoryJSON.object(result.json, maximumBytes: 1_048_576)
  }

  func mutate(_ capability: String, input: [String: Any]) async throws -> (
    AppCapabilityPlan, AutomationResult
  ) {
    let operation = UUID()
    let capability = "story.authoring." + capability
    let plan = try await provider.prepare(
      capabilityID: capability,
      inputJSON: StoryJSON.encode(input, maximumBytes: 65_536), operationID: operation)
    let result = try await provider.execute(
      capabilityID: capability, plan: plan,
      idempotencyKey: "synthetic-approval-\(operation.uuidString)", operationID: operation)
    try #require(
      result.outcome == .succeeded, "Native provider mutation did not confirm: \(result.json)")
    return (plan, result)
  }
}

@Test func nativePublicOwnersPreserveCASCandidateReviewAndWorkspaceScope() async throws {
  let fixture = try NativeOwnerFixture()
  defer { fixture.cleanUp() }
  let initial = try await fixture.status()
  #expect((initial["brief_revision"] as? StoryJSONNumber)?.unsignedInteger == 0)
  let operation = UUID()
  let capability = "story.authoring.brief.update"
  let plan = try await fixture.provider.prepare(
    capabilityID: capability,
    inputJSON: #"{"expected_revision":0,"brief":{"genre":"synthetic"}}"#, operationID: operation)
  let preview = try StoryJSON.object(plan.previewJSON, maximumBytes: 262_144)
  #expect(preview["project_id"] as? String == "native-provider")
  #expect((try await fixture.status()["brief_revision"] as? StoryJSONNumber)?.unsignedInteger == 0)
  let applied = try await fixture.provider.execute(
    capabilityID: capability, plan: plan,
    idempotencyKey: "synthetic-brief", operationID: operation)
  #expect(applied.outcome == .succeeded)
  #expect(
    try await fixture.provider.execute(
      capabilityID: capability, plan: plan,
      idempotencyKey: "synthetic-brief", operationID: operation) == applied)
  #expect((try await fixture.status()["brief_revision"] as? StoryJSONNumber)?.unsignedInteger == 1)

  let staleOperation = UUID()
  let stale = try await fixture.provider.prepare(
    capabilityID: capability,
    inputJSON: #"{"expected_revision":1,"brief":{"genre":"stale-proposal"}}"#,
    operationID: staleOperation)
  _ = try NativeOwnerFixture.run(
    fixture.storyctl,
    arguments: [
      "--story-root", fixture.root.path, "text", "--workspace", fixture.workspace.path,
      "--geultu-bin", fixture.geultu.path, "author", "authoring.brief", "--input", "-",
    ],
    input:
      #"{"command_id":"synthetic-other-brief","expected_project_id":"native-provider","expected_revision":1,"brief":{"genre":"other-writer"}}"#
  )
  let staleResult = try await fixture.provider.execute(
    capabilityID: capability, plan: stale,
    idempotencyKey: "synthetic-stale", operationID: staleOperation)
  #expect(staleResult.outcome != .succeeded, "Stale CAS result: \(staleResult.json)")
  let afterCAS = try await fixture.status()
  #expect((afterCAS["brief"] as? [String: Any])?["genre"] as? String == "other-writer")

  let (taskPlan, _) = try await fixture.mutate(
    "task.prepare",
    input: [
      "target": "story", "instruction": "Preserve the synthetic archive source.",
    ])
  let taskPreview = try StoryJSON.object(taskPlan.previewJSON, maximumBytes: 262_144)
  let taskID = try #require(taskPreview["task_id"] as? String)
  let draft: [String: Any] = [
    "blocks": [
      [
        "id": "main", "heading": "Archive",
        "text": "A synthetic author preserves a source in a closed archive.",
      ]
    ],
    "change_summary": "Synthetic owner boundary test", "assumptions": [String](),
    "open_questions": [String](),
  ]
  let (firstCandidate, _) = try await fixture.mutate(
    "candidate.submit", input: ["task_id": taskID, "draft": draft])
  let candidateID = try #require(
    StoryJSON.object(firstCandidate.previewJSON, maximumBytes: 262_144)["candidate_id"] as? String)
  let (_, rejected) = try await fixture.mutate(
    "candidate.review", input: ["candidate_id": candidateID, "accept": false])
  let rejection = try StoryJSON.object(rejected.json, maximumBytes: 1_048_576)
  let rejectedCandidate = (rejection["result"] as? [String: Any])?["candidate"] as? [String: Any]
  #expect(
    ((rejectedCandidate?["decision"] as? [String: Any])?["accepted"] as? NSNumber)?.boolValue
      == false)
  let rejectedStatus = try await fixture.status()
  #expect(
    (rejectedStatus["documents"] as? [[String: Any]])?.first(where: {
      $0["kind"] as? String == "story"
    })?["status"] as? String == "missing")

  let (secondCandidate, _) = try await fixture.mutate(
    "candidate.submit", input: ["task_id": taskID, "draft": draft])
  let secondID = try #require(
    StoryJSON.object(secondCandidate.previewJSON, maximumBytes: 262_144)["candidate_id"] as? String)
  _ = try await fixture.mutate(
    "candidate.review", input: ["candidate_id": secondID, "accept": true])
  let adoptedStatus = try await fixture.status()
  #expect(
    (adoptedStatus["documents"] as? [[String: Any]])?.first(where: {
      $0["kind"] as? String == "story"
    })?["status"] as? String == "current")

  let scopeOperation = UUID()
  let scopePlan = try await fixture.provider.prepare(
    capabilityID: capability,
    inputJSON: #"{"expected_revision":2,"brief":{"genre":"wrong-scope"}}"#,
    operationID: scopeOperation)
  let descriptor = fixture.workspace.appending(path: "story-project.json")
  let original = try Data(contentsOf: descriptor)
  defer { try? original.write(to: descriptor, options: .atomic) }
  var changed = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
  changed["projectId"] = "different-project"
  try JSONSerialization.data(withJSONObject: changed).write(to: descriptor, options: .atomic)
  let scopeResult = try await fixture.provider.execute(
    capabilityID: capability, plan: scopePlan,
    idempotencyKey: "synthetic-scope", operationID: scopeOperation)
  #expect(scopeResult.outcome == .failed)
  try original.write(to: descriptor, options: .atomic)
  let final = try await fixture.status()
  #expect((final["brief_revision"] as? StoryJSONNumber)?.unsignedInteger == 2)
  #expect((final["brief"] as? [String: Any])?["genre"] as? String == "other-writer")
}
