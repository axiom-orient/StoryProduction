import Foundation
import SEMIControl
import SEMICore
import Testing

@testable import StoryProductionSEMIProvider

private struct ProviderFixture {
  let root: URL
  let workspace: URL
  let provider: StoryProvider

  init(revision: String = "0") throws {
    root = FileManager.default.temporaryDirectory.appending(
      path: "story-provider-\(UUID().uuidString)"
    )
    .resolvingSymlinksInPath()
    workspace = root.appending(path: "workspace")
    try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true)
    let cli = root.appending(path: "storyctl-fixture")
    let script = #"""
      #!/bin/sh
      operation="$9"
      /bin/cat > ".request.$operation.json"
      printf '%s\n' "$operation" >> .calls
      case "$operation" in
        authoring.status)
          if [ -f .block-read ]; then
            printf 'started' > .read-started
            /bin/sleep 30
          fi
          /bin/cat .status.json ;;
        authoring.task) /bin/cat .task.json ;;
        authoring.candidate) /bin/cat .candidate.json ;;
        *)
          if [ -f .return-on-termination ]; then
            trap '/bin/sleep 0.2; /bin/cat .mutation.json; exit 0' TERM
            printf 'started' > .dispatch-started
            /bin/sleep 30
          fi
          if [ -f .block-dispatch ]; then
            printf 'started' > .dispatch-started
            /bin/sleep 30
          fi
          if [ -f .mutation-error ]; then
            /bin/cat .mutation-error >&2
            exit 2
          fi
          /bin/cat .mutation.json
          ;;
      esac
      """#
    try script.write(to: cli, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: cli.path)
    let status = """
      {"project_id":"fixture-project","project_revision":4,"brief_revision":\(revision),"brief":{},"documents":[{"kind":"source","status":"current","revision":1},{"kind":"story","status":"missing","revision":0},{"kind":"synopsis","status":"missing","revision":0}]}
      """
    try status.write(
      to: workspace.appending(path: ".status.json"), atomically: true, encoding: .utf8)
    try
      #"{"commit":{"project_id":"fixture-project","revision":5,"event_count":1,"idempotent":false},"result":{"project_id":"fixture-project","project_revision":5,"brief_revision":1,"brief":{"genre":"fixture"},"documents":[]}}"#
      .write(
        to: workspace.appending(path: ".mutation.json"), atomically: true, encoding: .utf8)
    let configuration = try ProviderConfiguration.parse([
      "--story-root", root.path, "--workspace", workspace.path,
      "--storyctl-bin", cli.path, "--geultu-bin", "/usr/bin/true", "--instance-id",
      UUID().uuidString,
    ])
    provider = StoryProvider(configuration: configuration)
  }

  func cleanUp() { try? FileManager.default.removeItem(at: root) }
  func write(_ text: String, to name: String) throws {
    try text.write(to: workspace.appending(path: name), atomically: true, encoding: .utf8)
  }
  func calls() -> [String] {
    (try? String(contentsOf: workspace.appending(path: ".calls"), encoding: .utf8))?
      .split(separator: "\n").map(String.init) ?? []
  }
  func prepareBrief(revision: String = "0", operationID: UUID) async throws -> AppCapabilityPlan {
    try await provider.prepare(
      capabilityID: "story.authoring.brief.update",
      inputJSON: "{\"expected_revision\":\(revision),\"brief\":{\"genre\":\"fixture\"}}",
      operationID: operationID)
  }
}

@Test(arguments: [
  "18446744073709551616", "18446744073709551617", "1e20", "true", "-1",
  "9007199254740993.1", "1.000000000000000000000000000000000000000001",
])
func invalidBriefCASIsRejectedBeforeAnyOwnerCall(revision: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  await #expect(throws: (any Error).self) {
    try await fixture.prepareBrief(revision: revision, operationID: UUID())
  }
  #expect(fixture.calls().isEmpty)
}

@Test(arguments: ["9007199254740993e0", "18446744073709551615"])
func exactCASValueReachesOwnerWithoutLossAndReplayDoesNotRedispatch(revision: String) async throws {
  let fixture = try ProviderFixture(revision: revision)
  defer { fixture.cleanUp() }
  let operation = UUID()
  let plan = try await fixture.prepareBrief(revision: revision, operationID: operation)
  let expected = try #require(StoryJSONNumber(token: revision).unsignedInteger)
  #expect(plan.target.revision == String(expected))
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved-exact", operationID: operation)
  #expect(result.outcome == .succeeded)
  let sent = try String(
    contentsOf: fixture.workspace.appending(path: ".request.authoring.brief.json"), encoding: .utf8)
  let owner = try StoryJSON.object(sent, maximumBytes: 65_536)
  #expect((owner["expected_revision"] as? StoryJSONNumber)?.unsignedInteger == expected)
  #expect(owner["expected_project_id"] as? String == "fixture-project")
  let replay = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved-exact", operationID: operation)
  #expect(replay == result)
  #expect(fixture.calls() == ["authoring.status", "authoring.brief"])
}

@Test(arguments: [
  #"{"expected_revision":1,"expected_revision":0,"brief":{"genre":"x"}}"#,
  #"{"expected_revision":0,"brief":{"tone":"first","tone":"last"}}"#,
  #"{"expected_revision":0,"brief":{"genre":4}}"#,
  #"{"expected_revision":0,"brief":{"avoid":[1]}}"#,
])
func invalidNestedOrDuplicateInputNeverPreparesAPlan(input: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  await #expect(throws: (any Error).self) {
    try await fixture.provider.prepare(
      capabilityID: "story.authoring.brief.update", inputJSON: input,
      operationID: UUID())
  }
  #expect(fixture.calls().isEmpty)
}

@Test(arguments: ["18446744073709551616", "9007199254740993.1"])
func malformedOwnerRevisionCannotProduceAReviewablePlan(revision: String) async throws {
  let fixture = try ProviderFixture(revision: revision)
  defer { fixture.cleanUp() }
  await #expect(throws: (any Error).self) {
    try await fixture.prepareBrief(operationID: UUID())
  }
  #expect(fixture.calls() == ["authoring.status"])
}

@Test(arguments: [
  (#"{"status":"not_applied"}"#, AutomationResult.Outcome.failed),
  (#"{"status":"not_started"}"#, AutomationResult.Outcome.failed),
  (#"{"status":"source_changed"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"outcome_unknown"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"cancelled"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"failed"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"source_stale"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"arbitrary"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"applied"}"#, AutomationResult.Outcome.uncertain),
  ("{}", AutomationResult.Outcome.uncertain),
  (
    #"{"commit":{"project_id":"different-project","revision":5,"event_count":1,"idempotent":false},"result":{}}"#,
    AutomationResult.Outcome.uncertain
  ),
  (
    #"{"commit":{"project_id":"fixture-project","revision":18446744073709551616,"event_count":1,"idempotent":false},"result":{}}"#,
    AutomationResult.Outcome.uncertain
  ),
  (#"{"status":"applied","status":"not_applied"}"#, AutomationResult.Outcome.uncertain),
  (#"{"status":"outcome_unknown","status":"applied"}"#, AutomationResult.Outcome.uncertain),
  (
    #"{"commit":{"project_id":"fixture-project","revision":0,"event_count":1,"idempotent":false},"result":{"project_id":"fixture-project","project_revision":0,"brief_revision":1,"brief":{"genre":"fixture"},"documents":[]}}"#,
    AutomationResult.Outcome.uncertain
  ),
  ("[]", AutomationResult.Outcome.uncertain),
])
func exitZeroCannotTurnFailureOrUncertaintyIntoSuccess(
  output: String, outcome: AutomationResult.Outcome
) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write(output, to: ".mutation.json")
  let operation = UUID()
  let plan = try await fixture.prepareBrief(operationID: operation)
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == outcome)
  #expect(fixture.calls() == ["authoring.status", "authoring.brief"])
}

@Test func alteredPlanAndInvalidKeyDoNotConsumeThePreparedOperation() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let operation = UUID()
  let plan = try await fixture.prepareBrief(operationID: operation)
  let changed = AppCapabilityPlan(token: plan.token, target: plan.target, previewJSON: "{}")
  let rejected = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: changed, idempotencyKey: "approved", operationID: operation)
  #expect(rejected.outcome == .failed)
  let invalidKey = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "", operationID: operation)
  #expect(invalidKey.outcome == .failed)
  #expect(fixture.calls() == ["authoring.status"])
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == .succeeded)
}

@Test func cancellationAfterDispatchPreservesUncertaintyAndDoesNotReplay() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write("", to: ".block-dispatch")
  let operation = UUID()
  let plan = try await fixture.prepareBrief(operationID: operation)
  let executing = Task {
    try await fixture.provider.execute(
      capabilityID: "story.authoring.brief.update",
      plan: plan, idempotencyKey: "approved", operationID: operation)
  }
  let started = fixture.workspace.appending(path: ".dispatch-started")
  let deadline = ContinuousClock.now.advanced(by: .seconds(5))
  while !FileManager.default.fileExists(atPath: started.path), ContinuousClock.now < deadline {
    try await Task.sleep(for: .milliseconds(10))
  }
  try #require(FileManager.default.fileExists(atPath: started.path))
  await fixture.provider.cancel(operationID: operation)
  let result = try await executing.value
  #expect(result.outcome == .uncertain)
  let replay = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(replay == result)
  #expect(fixture.calls() == ["authoring.status", "authoring.brief"])
}

@Test func cancelledReadyPlanNeverDispatchesAnOwnerMutation() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let operation = UUID()
  let plan = try await fixture.prepareBrief(operationID: operation)
  await fixture.provider.cancel(operationID: operation)
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == .failed)
  #expect(fixture.calls() == ["authoring.status"])
}

@Test(arguments: [
  #"{"candidate_id":"selected","accept":true,"note":1}"#,
  #"{"candidate_id":"selected","accept":true,"note":null}"#,
  #"{"candidate_id":"selected","accept":1}"#,
  #"{"task_id":"task","draft":{}}"#,
])
func invalidReviewAndDraftTypesAreRejectedBeforeReadingTheOwner(input: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let capability =
    input.contains("task_id")
    ? "story.authoring.candidate.submit" : "story.authoring.candidate.review"
  await #expect(throws: (any Error).self) {
    try await fixture.provider.prepare(
      capabilityID: capability, inputJSON: input, operationID: UUID())
  }
  #expect(fixture.calls().isEmpty)
}

@Test(arguments: ["{}", #"{"blocked":true}"#, #"[1]"#])
func malformedOwnerBlockersNeverBecomeAnEmptyApprovalList(blockers: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let hash = String(repeating: "a", count: 64)
  try fixture.write(
    """
    {"candidate_hash":"\(hash)","adoption_blockers":\(blockers),"candidate":{"id":"selected","task_id":"task","based_on":null,"decision":null,"draft":{"blocks":[{"id":"main","heading":"Source","text":"Text"}],"change_summary":"Fixture","assumptions":[],"open_questions":[]}}}
    """, to: ".candidate.json")
  await #expect(throws: (any Error).self) {
    try await fixture.provider.prepare(
      capabilityID: "story.authoring.candidate.review",
      inputJSON: #"{"candidate_id":"selected","accept":true}"#, operationID: UUID())
  }
  #expect(fixture.calls() == ["authoring.status", "authoring.candidate"])
}

@Test func ownerDuplicateKeysNeverCreateAPlan() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write(
    #"{"project_id":"fixture-project","brief_revision":1,"brief_revision":0,"brief":{}}"#,
    to: ".status.json")
  await #expect(throws: StoryJSONError.duplicateKey) {
    try await fixture.prepareBrief(operationID: UUID())
  }
  #expect(fixture.calls() == ["authoring.status"])
}

@Test func duplicateDispatchAndReadCannotReleaseTheFirstCallsCancellation() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write("", to: ".return-on-termination")
  let operation = UUID()
  let capability = "story.authoring.brief.update"
  let plan = try await fixture.prepareBrief(operationID: operation)
  let executing = Task {
    try await fixture.provider.execute(
      capabilityID: capability, plan: plan,
      idempotencyKey: "first-approved-key", operationID: operation)
  }
  let started = fixture.workspace.appending(path: ".dispatch-started")
  let deadline = ContinuousClock.now.advanced(by: .seconds(5))
  while !FileManager.default.fileExists(atPath: started.path), ContinuousClock.now < deadline {
    try await Task.sleep(for: .milliseconds(10))
  }
  try #require(FileManager.default.fileExists(atPath: started.path))
  let cancellation = Task { await fixture.provider.cancel(operationID: operation) }
  await Task.yield()
  let duplicate = try await fixture.provider.execute(
    capabilityID: capability, plan: plan,
    idempotencyKey: "second-key", operationID: operation)
  #expect(duplicate.outcome == .uncertain)
  let invalid = try await fixture.provider.execute(
    capabilityID: capability, plan: plan,
    idempotencyKey: "", operationID: operation)
  #expect(invalid.outcome == .failed)
  let read = try await fixture.provider.read(
    capabilityID: "story.authoring.status", inputJSON: "{}",
    operationID: operation)
  #expect(read.outcome == .failed)
  await #expect(throws: (any Error).self) { try await fixture.prepareBrief(operationID: operation) }
  await cancellation.value
  let original = try await executing.value
  #expect(original.outcome == .uncertain)
  #expect(fixture.calls() == ["authoring.status", "authoring.brief"])
  #expect(
    try await fixture.provider.execute(
      capabilityID: capability, plan: plan,
      idempotencyKey: "first-approved-key", operationID: operation) == original)
}

@Test func activeReadOwnsItsOperationIDUntilItDrains() async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write("", to: ".block-read")
  let operation = UUID()
  let reading = Task {
    try await fixture.provider.read(
      capabilityID: "story.authoring.status", inputJSON: "{}",
      operationID: operation)
  }
  let started = fixture.workspace.appending(path: ".read-started")
  let deadline = ContinuousClock.now.advanced(by: .seconds(5))
  while !FileManager.default.fileExists(atPath: started.path), ContinuousClock.now < deadline {
    try await Task.sleep(for: .milliseconds(10))
  }
  try #require(FileManager.default.fileExists(atPath: started.path))
  await #expect(throws: (any Error).self) { try await fixture.prepareBrief(operationID: operation) }
  let duplicate = try await fixture.provider.read(
    capabilityID: "story.authoring.status", inputJSON: "{}",
    operationID: operation)
  #expect(duplicate.outcome == .failed)
  await fixture.provider.cancel(operationID: operation)
  await #expect(throws: CancellationError.self) { try await reading.value }
  #expect(fixture.calls() == ["authoring.status"])
}

@Test(arguments: ["matching", "wrong-task", "wrong-draft"])
func submissionReceiptMustMatchTheApprovedTaskAndDraft(variation: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  try fixture.write(
    #"{"task":{"id":"selected-task","target":"story"},"adoption_blockers":[]}"#,
    to: ".task.json")
  let draft: [String: Any] = [
    "blocks": [["id": "main", "heading": "Source", "text": "Approved text"]],
    "change_summary": "Approved draft", "assumptions": [String](), "open_questions": [String](),
  ]
  let operation = UUID()
  let plan = try await fixture.provider.prepare(
    capabilityID: "story.authoring.candidate.submit",
    inputJSON: StoryJSON.encode(
      ["task_id": "selected-task", "draft": draft], maximumBytes: 65_536),
    operationID: operation)
  let candidateID = try #require(
    StoryJSON.object(plan.previewJSON, maximumBytes: 262_144)["candidate_id"] as? String)
  var returnedDraft = draft
  if variation == "wrong-draft" { returnedDraft["change_summary"] = "Different draft" }
  let response: [String: Any] = [
    "commit": [
      "project_id": "fixture-project", "revision": 5, "event_count": 1, "idempotent": false,
    ],
    "result": [
      "candidate": [
        "id": candidateID, "task_id": variation == "wrong-task" ? "other-task" : "selected-task",
        "draft": returnedDraft, "decision": NSNull(), "based_on": NSNull(),
      ],
      "candidate_hash": String(repeating: "a", count: 64), "adoption_blockers": [String](),
    ],
  ]
  try fixture.write(StoryJSON.encode(response, maximumBytes: 65_536), to: ".mutation.json")
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.candidate.submit",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == (variation == "matching" ? .succeeded : .uncertain))
}

@Test(arguments: ["matching", "wrong-instruction", "cancelled"])
func preparedTaskReceiptMustMatchApprovedInstructionAndRemainActive(variation: String) async throws
{
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let operation = UUID()
  let plan = try await fixture.provider.prepare(
    capabilityID: "story.authoring.task.prepare",
    inputJSON: #"{"target":"story","instruction":"Approved instruction"}"#, operationID: operation)
  let taskID = try #require(
    StoryJSON.object(plan.previewJSON, maximumBytes: 262_144)["task_id"] as? String)
  let response: [String: Any] = [
    "commit": [
      "project_id": "fixture-project", "revision": 5, "event_count": 1, "idempotent": false,
    ],
    "result": [
      "task": [
        "id": taskID, "target": "story",
        "instruction": variation == "wrong-instruction"
          ? "Different instruction" : "Approved instruction",
        "selection": [String](), "cancelled": variation == "cancelled",
      ], "adoption_blockers": [String](),
    ],
  ]
  try fixture.write(StoryJSON.encode(response, maximumBytes: 65_536), to: ".mutation.json")
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.task.prepare",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == (variation == "matching" ? .succeeded : .uncertain))
}

@Test(arguments: ["matching", "wrong-tone"])
func briefReceiptConfirmsProvidedFieldsAndAllowsOwnerDefaults(variation: String) async throws {
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let operation = UUID()
  let plan = try await fixture.provider.prepare(
    capabilityID: "story.authoring.brief.update",
    inputJSON: #"{"expected_revision":0,"brief":{"tone":"Approved tone"}}"#, operationID: operation)
  let response: [String: Any] = [
    "commit": [
      "project_id": "fixture-project", "revision": 5, "event_count": 1, "idempotent": false,
    ],
    "result": [
      "project_id": "fixture-project", "project_revision": 5, "brief_revision": 1,
      "brief": [
        "genre": "", "tone": variation == "wrong-tone" ? "Different tone" : "Approved tone",
      ],
      "documents": [[String: Any]](),
    ],
  ]
  try fixture.write(StoryJSON.encode(response, maximumBytes: 65_536), to: ".mutation.json")
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.brief.update",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == (variation == "matching" ? .succeeded : .uncertain))
}

@Test(arguments: ["matching", "wrong-draft", "wrong-task", "wrong-lineage"])
func reviewReceiptRetainsTheExactPreviewedCandidateDespiteTheSameClaimedHash(variation: String)
  async throws
{
  let fixture = try ProviderFixture()
  defer { fixture.cleanUp() }
  let hash = String(repeating: "a", count: 64)
  let draft: [String: Any] = [
    "blocks": [["id": "main", "heading": "Source", "text": "Exact preview text"]],
    "change_summary": "Approved candidate", "assumptions": [String](), "open_questions": [String](),
  ]
  var candidate: [String: Any] = [
    "id": "selected", "task_id": "selected-task", "based_on": "parent",
    "decision": NSNull(), "draft": draft,
  ]
  try fixture.write(
    StoryJSON.encode(
      [
        "candidate": candidate, "candidate_hash": hash,
        "adoption_blockers": [String](),
      ], maximumBytes: 65_536), to: ".candidate.json")
  let operation = UUID()
  let plan = try await fixture.provider.prepare(
    capabilityID: "story.authoring.candidate.review",
    inputJSON: #"{"candidate_id":"selected","accept":true}"#, operationID: operation)
  if variation == "wrong-task" { candidate["task_id"] = "different-task" }
  if variation == "wrong-lineage" { candidate["based_on"] = "different-parent" }
  if variation == "wrong-draft" {
    var differentDraft = draft
    differentDraft["change_summary"] = "Different candidate content"
    candidate["draft"] = differentDraft
  }
  candidate["decision"] = ["actor": "SEMI", "accepted": true, "candidate_hash": hash, "note": ""]
  let receipt: [String: Any] = [
    "commit": [
      "project_id": "fixture-project", "revision": 5, "event_count": 1, "idempotent": false,
    ],
    "result": ["candidate": candidate, "candidate_hash": hash, "adoption_blockers": [String]()],
  ]
  try fixture.write(StoryJSON.encode(receipt, maximumBytes: 65_536), to: ".mutation.json")
  let result = try await fixture.provider.execute(
    capabilityID: "story.authoring.candidate.review",
    plan: plan, idempotencyKey: "approved", operationID: operation)
  #expect(result.outcome == (variation == "matching" ? .succeeded : .uncertain))
}
