import CoreFoundation
import Foundation
import SEMICore
import SEMIControl

actor StoryProvider: AppCapabilityProvider {
  private static let emptySchema = #"{"type":"object","properties":{},"additionalProperties":false}"#
  private static let documentSchema = #"{"type":"object","required":["kind"],"properties":{"kind":{"type":"string","enum":["source","story","synopsis"]}},"additionalProperties":false}"#
  private static let itemSchema = #"{"type":"object","required":["id"],"properties":{"id":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"}},"additionalProperties":false}"#
  private static let taskPrepareSchema = #"{"type":"object","required":["target","instruction"],"properties":{"target":{"type":"string","enum":["story","synopsis"]},"instruction":{"type":"string","minLength":1,"maxLength":8192},"selection":{"type":"array","maxItems":512,"uniqueItems":true,"items":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"}}},"additionalProperties":false}"#
  private static let briefUpdateSchema = #"{"type":"object","required":["expected_revision","brief"],"properties":{"expected_revision":{"type":"integer","minimum":0,"maximum":18446744073709551615},"brief":{"type":"object","additionalProperties":false,"properties":{"audience":{"type":"string"},"genre":{"type":"string"},"tone":{"type":"string"},"point_of_view":{"type":"string"},"tense":{"type":"string"},"voice_notes":{"type":"string"},"ending":{"type":"string"},"preserve":{"type":"array","items":{"type":"string"}},"avoid":{"type":"array","items":{"type":"string"}}}}},"additionalProperties":false}"#
  private static let candidateSubmitSchema = #"{"type":"object","required":["task_id","draft"],"properties":{"task_id":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"},"based_on":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"},"draft":{"type":"object","required":["blocks","change_summary","assumptions","open_questions"],"additionalProperties":false,"properties":{"blocks":{"type":"array","minItems":1,"maxItems":512,"items":{"type":"object","required":["id","heading","text"],"additionalProperties":false,"properties":{"id":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"},"heading":{"type":"string","minLength":1},"text":{"type":"string","minLength":1}}}},"change_summary":{"type":"string","minLength":1},"assumptions":{"type":"array","items":{"type":"string"}},"open_questions":{"type":"array","items":{"type":"string"}}}}},"additionalProperties":false}"#
  private static let candidateReviewSchema = #"{"type":"object","required":["candidate_id","accept"],"properties":{"candidate_id":{"type":"string","pattern":"^[A-Za-z0-9_.-]{1,64}$"},"accept":{"type":"boolean"},"note":{"type":"string","maxLength":4096}},"additionalProperties":false}"#

  private static let maximumOwnerProcesses = 4
  private static let maximumPreparedOperations = 32
  private static let briefFields: Set<String> = [
    "audience", "genre", "tone", "point_of_view", "tense", "voice_notes", "ending",
    "preserve", "avoid",
  ]

  private struct Prepared: Sendable, Equatable {
    let capabilityID: String
    let inputJSON: String
    let ownerOperation: String
    let ownerCommandID: String
    let ownerInputJSON: String
    let plan: AppCapabilityPlan
  }

  private enum Slot {
    case preparing
    case ready(Prepared)
    case dispatching(Prepared, String)
    case completed(Prepared, String, AutomationResult)
  }

  private struct OwnerProcess: Sendable {
    let cancel: @Sendable () -> Void
    let drain: @Sendable () async -> Void
  }

  private let cli: StoryCLI
  private var slots: [UUID: Slot] = [:]
  private var completedOrder: [UUID] = []
  private var ownerProcesses: [UUID: OwnerProcess] = [:]
  private var cancelledOperations = Set<UUID>()

  init(configuration: ProviderConfiguration) {
    cli = StoryCLI(configuration: configuration)
  }

  static var capabilities: [AppCapabilityDescriptor] {
    [
      AppCapabilityDescriptor(
        id: "story.authoring.status",
        summary: "Read document freshness, pending candidates, and next actions in the workspace selected when this provider started.",
        kind: .read,
        risk: "local_story_metadata_read",
        permissions: ["story.workspace.read"],
        schemaJSON: emptySchema,
        failureSemantics: "The StoryProduction workspace is unchanged; unavailable or malformed owner output is a read failure.",
        recoverySemantics: "Retry the same bounded read after the selected workspace and geultu binary are available."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.document.read",
        summary: "Read one current source, story, or synopsis document from the selected StoryProduction workspace.",
        kind: .read,
        risk: "local_story_content_read",
        permissions: ["story.workspace.read"],
        schemaJSON: documentSchema,
        failureSemantics: "The StoryProduction workspace is unchanged; stale reasons are returned by geultu.",
        recoverySemantics: "Read the exact current document kind again after resolving owner-reported stale or missing state."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.candidate.read",
        summary: "Read one exact draft candidate and its source-binding blockers from the selected workspace.",
        kind: .read,
        risk: "local_story_content_read",
        permissions: ["story.workspace.read"],
        schemaJSON: itemSchema,
        failureSemantics: "The candidate is not modified or adopted; a missing candidate is a read failure.",
        recoverySemantics: "Use a candidate ID returned by story.authoring.status."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.task.read",
        summary: "Read one exact saved writing packet from the selected workspace without preparing a new task or calling a writer.",
        kind: .read,
        risk: "local_story_content_read",
        permissions: ["story.workspace.read"],
        schemaJSON: itemSchema,
        failureSemantics: "The task is not modified; a missing or stale packet is a read failure.",
        recoverySemantics: "Use a task ID returned by the current authoring status or candidate."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.task.prepare",
        summary: "Create an exact-approved, source-bound writing task in the selected workspace. It does not call a writer or change canonical story text.",
        kind: .mutation,
        risk: "story_authoring_task_write",
        permissions: ["story.workspace.write"],
        schemaJSON: taskPrepareSchema,
        failureSemantics: "The owner compares the approved project revision under its journal lock before creating the task. A lost post-dispatch result remains uncertain.",
        recoverySemantics: "Read the task ID from the approved preview/status before creating another task."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.brief.update",
        summary: "Update the selected workspace's writing brief at one exact brief revision.",
        kind: .mutation,
        risk: "story_guidance_write",
        permissions: ["story.workspace.write"],
        schemaJSON: briefUpdateSchema,
        failureSemantics: "The geultu brief revision is checked by the owner. A lost post-dispatch result remains uncertain.",
        recoverySemantics: "Read authoring status before planning any new brief update."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.candidate.submit",
        summary: "Save one exact untrusted draft against an existing writing task; this does not adopt it.",
        kind: .mutation,
        risk: "story_candidate_write",
        permissions: ["story.workspace.write"],
        schemaJSON: candidateSubmitSchema,
        failureSemantics: "Submission creates only an untrusted candidate. A lost post-dispatch result remains uncertain.",
        recoverySemantics: "Read the exact task and candidate in StoryProduction before submitting a new operation."
      ),
      AppCapabilityDescriptor(
        id: "story.authoring.candidate.review",
        summary: "Accept or reject one exact candidate after previewing its content and adoption blockers.",
        kind: .mutation,
        risk: "story_document_adoption",
        permissions: ["story.workspace.write"],
        schemaJSON: candidateReviewSchema,
        failureSemantics: "The geultu owner rechecks candidate hash, source, and guidance. A lost post-dispatch result remains uncertain.",
        recoverySemantics: "Read candidate and authoring status before deciding whether any new review is needed."
      ),
    ]
  }

  func read(capabilityID: String, inputJSON: String, operationID: UUID) async throws -> AutomationResult {
    guard let capability = Self.capabilities.first(where: { $0.id == capabilityID }),
      capability.kind == .read
    else { throw ControlPlaneError.unknownCommand(capabilityID) }
    guard slots[operationID] == nil, ownerProcesses[operationID] == nil,
      !cancelledOperations.contains(operationID)
    else { return try failed("story_operation_in_flight") }
    guard ownerProcesses.count < Self.maximumOwnerProcesses else {
      return try failed("story_read_capacity")
    }
    let operation: String
    let input: [String: Any]
    do {
      (operation, input) = try readRequest(capabilityID: capabilityID, inputJSON: inputJSON)
    } catch {
      return try failed("invalid_story_read")
    }

    defer { cancelledOperations.remove(operationID) }
    do {
      let json = try await ownerRead(operation, input: input, operationID: operationID)
      try ensureNotCancelled(operationID)
      let result = capabilityID == "story.authoring.candidate.read"
        ? candidatePresentation(json) : json
      return AutomationResult(
        outcome: .succeeded,
        json: try encode(result, maximumBytes: ControlPlanePolicy.standard.maximumResultBytes))
    } catch is CancellationError {
      throw CancellationError()
    } catch ProviderConfigurationError.workspaceChanged,
      ProviderConfigurationError.workspaceChangedDuringOwnerCall
    {
      return try failed("story_workspace_changed", operationID: operationID)
    } catch ProviderConfigurationError.workspaceSpawnActionFailed(_) {
      return try failed("story_workspace_spawn_setup_failed", operationID: operationID)
    } catch {
      if Task.isCancelled || cancelledOperations.contains(operationID) { throw CancellationError() }
      return try failed("story_read_failed")
    }
  }

  func prepare(capabilityID: String, inputJSON: String, operationID: UUID) async throws -> AppCapabilityPlan {
    guard let capability = Self.capabilities.first(where: { $0.id == capabilityID }),
      capability.kind == .mutation
    else { throw ControlPlaneError.unknownCommand(capabilityID) }
    guard slots[operationID] == nil, ownerProcesses[operationID] == nil,
      !cancelledOperations.contains(operationID)
    else {
      throw ControlPlaneError.stalePlan("Duplicate StoryProduction operation ID.")
    }
    try reserve(operationID)
    defer { cancelledOperations.remove(operationID) }

    do {
      let input = try object(inputJSON)
      let prepared: Prepared
      switch capabilityID {
      case "story.authoring.task.prepare":
        prepared = try await prepareTask(input, inputJSON: inputJSON, operationID: operationID)
      case "story.authoring.brief.update":
        prepared = try await prepareBrief(input, inputJSON: inputJSON, operationID: operationID)
      case "story.authoring.candidate.submit":
        prepared = try await prepareCandidateSubmission(input, inputJSON: inputJSON, operationID: operationID)
      case "story.authoring.candidate.review":
        prepared = try await prepareCandidateReview(input, inputJSON: inputJSON, operationID: operationID)
      default:
        throw ControlPlaneError.unknownCommand(capabilityID)
      }
      try ensurePreparing(operationID)
      slots[operationID] = .ready(prepared)
      return prepared.plan
    } catch {
      slots.removeValue(forKey: operationID)
      throw error
    }
  }

  func execute(
    capabilityID: String, plan: AppCapabilityPlan, idempotencyKey: String, operationID: UUID
  ) async throws -> AutomationResult {
    guard !idempotencyKey.isEmpty,
      idempotencyKey.utf8.count <= ControlPlanePolicy.standard.maximumIdempotencyKeyBytes
    else { return try failed("invalid_story_idempotency_key") }

    let prepared: Prepared
    switch slots[operationID] {
    case .some(.ready(let value)):
      guard value.capabilityID == capabilityID, value.plan == plan else {
        return try failed("story_candidate_mismatch")
      }
      prepared = value
    case .some(.dispatching(let value, _)):
      return try uncertain("story_operation_in_flight", prepared: value, operationID: operationID)
    case .some(.completed(let value, let previousKey, let result)):
      guard value.capabilityID == capabilityID, value.plan == plan, previousKey == idempotencyKey else {
        return try failed("story_idempotency_conflict")
      }
      return result
    default:
      return try failed("story_operation_not_prepared")
    }

    // Only the invocation owning this ready-to-dispatched transition can release cancellation.
    defer { cancelledOperations.remove(operationID) }

    guard !Task.isCancelled, !cancelledOperations.contains(operationID) else {
      let result = try failed("story_cancelled_before_dispatch")
      finish(prepared, key: idempotencyKey, result: result, operationID: operationID)
      return result
    }
    slots[operationID] = .dispatching(prepared, idempotencyKey)

    let result: AutomationResult
    do {
      let cliResult = try await runProcess(operationID: operationID) {
        try await self.cli.execute(
          operation: prepared.ownerOperation, inputJSON: prepared.ownerInputJSON)
      }
      if Task.isCancelled || cancelledOperations.contains(operationID) {
        result = try uncertain(
          "story_cancelled_after_dispatch", prepared: prepared, operationID: operationID)
      } else if cliResult.succeeded, let output = cliResult.standardOutput,
        let value = try? StoryJSON.object(
          output, maximumBytes: ControlPlanePolicy.standard.maximumResultBytes)
      {
        let outcome: AutomationResult.Outcome
        switch value["status"] as? String {
        case "not_applied", "not_started": outcome = .failed
        case "outcome_unknown", "uncertain", "cancelled": outcome = .uncertain
        default:
          outcome = confirmedMutation(value, prepared: prepared) ? .succeeded : .uncertain
        }
        result = AutomationResult(outcome: outcome, json: output)
      } else if !cliResult.succeeded, let output = cliResult.standardError,
        Self.isNotAppliedResponse(output)
      {
        result = AutomationResult(outcome: .failed, json: output)
      } else {
        result = try uncertain(
          "story_owner_result_unconfirmed", prepared: prepared, operationID: operationID)
      }
    } catch ProviderConfigurationError.workspaceChanged {
      result = try failed("story_workspace_changed", operationID: operationID)
    } catch ProviderConfigurationError.workspaceChangedDuringOwnerCall {
      result = try uncertain(
        "story_workspace_changed_during_dispatch", prepared: prepared, operationID: operationID)
    } catch ProviderConfigurationError.workspaceSpawnActionFailed(_) {
      result = try failed("story_workspace_spawn_setup_failed", operationID: operationID)
    } catch {
      if error as? StoryProviderError == .processCapacity {
        result = try failed("story_process_capacity", operationID: operationID)
      } else {
        result = try uncertain(
          "story_owner_dispatch_unconfirmed", prepared: prepared, operationID: operationID)
      }
    }
    finish(prepared, key: idempotencyKey, result: result, operationID: operationID)
    return result
  }

  func cancel(operationID: UUID) async {
    switch slots[operationID] {
    case .some(.preparing):
      cancelledOperations.insert(operationID)
      slots.removeValue(forKey: operationID)
      await cancelAndDrainOwnerProcess(operationID)
    case .some(.ready(_)):
      slots.removeValue(forKey: operationID)
    case .some(.dispatching(_, _)):
      cancelledOperations.insert(operationID)
      await cancelAndDrainOwnerProcess(operationID)
    case .some(.completed(_, _, _)), .none:
      guard ownerProcesses[operationID] != nil else { return }
      cancelledOperations.insert(operationID)
      await cancelAndDrainOwnerProcess(operationID)
    }
  }

  private func prepareBrief(
    _ input: [String: Any], inputJSON: String, operationID: UUID
  ) async throws -> Prepared {
    guard exactKeys(input, required: ["expected_revision", "brief"]),
      let expected = integer(input["expected_revision"]),
      let brief = input["brief"] as? [String: Any], validBrief(brief), !brief.isEmpty
    else { throw ControlPlaneError.invalidRegistration("Invalid brief update input.") }

    let status = try await ownerRead("authoring.status", input: [:], operationID: operationID)
    try ensurePreparing(operationID)
    guard let projectID = validID(status["project_id"]),
      let currentRevision = integer(status["brief_revision"]),
      let currentBrief = status["brief"] as? [String: Any], validBrief(currentBrief),
      currentRevision == expected
    else { throw ControlPlaneError.stalePlan("The StoryProduction brief revision changed before preparation.") }

    let commandID = ownerCommandID("brief", operationID)
    let ownerInput: [String: Any] = [
      "command_id": commandID, "expected_project_id": projectID,
      "expected_revision": expected, "brief": brief,
    ]
    let preview: [String: Any] = [
      "action": "update_story_brief", "project_id": projectID,
      "expected_brief_revision": expected, "current_brief": currentBrief,
      "new_brief": brief,
    ]
    return try makePrepared(
      capabilityID: "story.authoring.brief.update", inputJSON: inputJSON,
      ownerOperation: "authoring.brief", ownerInput: ownerInput,
      targetID: "\(projectID)/brief", targetRevision: String(expected), preview: preview)
  }

  private func prepareTask(
    _ input: [String: Any], inputJSON: String, operationID: UUID
  ) async throws -> Prepared {
    guard exactKeys(input, required: ["target", "instruction"], optional: ["selection"]),
      let target = input["target"] as? String, ["story", "synopsis"].contains(target),
      let instruction = input["instruction"] as? String,
      !instruction.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
      instruction.utf8.count <= 8_192
    else { throw ControlPlaneError.invalidRegistration("Invalid authoring task input.") }
    let selection: [String]
    if let rawSelection = input["selection"] {
      guard let values = rawSelection as? [String] else {
        throw ControlPlaneError.invalidRegistration("Invalid authoring task selection.")
      }
      selection = values
    } else {
      selection = []
    }
    guard selection.count <= 512, selection.allSatisfy({ validID($0) != nil }),
      Set(selection).count == selection.count
    else { throw ControlPlaneError.invalidRegistration("Invalid authoring task selection.") }

    let status = try await ownerRead("authoring.status", input: [:], operationID: operationID)
    try ensurePreparing(operationID)
    guard let projectID = validID(status["project_id"]),
      let projectRevision = integer(status["project_revision"]),
      let documents = status["documents"] as? [[String: Any]]
    else { throw ControlPlaneError.providerUnavailable("StoryProduction returned incomplete authoring status.") }
    func uniqueDocument(_ kind: String) -> [String: Any]? {
      let matches = documents.filter { $0["kind"] as? String == kind }
      return matches.count == 1 ? matches[0] : nil
    }
    guard let targetDocument = uniqueDocument(target),
      let sourceDocument = uniqueDocument("source"),
      let storyDocument = uniqueDocument("story")
    else { throw ControlPlaneError.providerUnavailable("StoryProduction returned ambiguous document status.") }

    let prerequisites = target == "story" ? [sourceDocument] : [sourceDocument, storyDocument]
    for prerequisite in prerequisites {
      guard prerequisite["status"] as? String == "current" else {
        throw ControlPlaneError.stalePlan("Refresh the prerequisites before preparing a \(target) task.")
      }
    }

    let targetStatus = targetDocument["status"] as? String ?? "missing"
    let documentStates: [[String: Any]] = documents.map { document in
      [
        "kind": document["kind"] as? String ?? "unknown",
        "status": document["status"] as? String ?? "unknown",
        "revision": document["revision"] ?? NSNull(),
      ]
    }
    if !selection.isEmpty {
      guard targetStatus == "current" else {
        throw ControlPlaneError.stalePlan("A partial task requires a current \(target) document.")
      }
      let blockIDs = Set(
        (targetDocument["blocks"] as? [[String: Any]] ?? []).compactMap { $0["id"] as? String })
      guard selection.allSatisfy(blockIDs.contains) else {
        throw ControlPlaneError.stalePlan("A selected block is no longer in the current document.")
      }
    }

    let taskID = "semi-task-\(operationID.uuidString.lowercased())"
    let commandID = ownerCommandID("prepare", operationID)
    let ownerInput: [String: Any] = [
      "command_id": commandID, "task_id": taskID, "target": target,
      "instruction": instruction, "selection": selection,
      "expected_project_id": projectID,
      "expected_project_revision": projectRevision,
    ]
    let preview: [String: Any] = [
      "action": "create_source_bound_authoring_task", "project_id": projectID,
      "expected_project_revision": projectRevision, "task_id": taskID,
      "target": target, "instruction": instruction, "selection": selection,
      "documents": documentStates,
      "brief_revision": status["brief_revision"] ?? NSNull(),
      "brief": status["brief"] ?? [:],
      "writer_invoked": false, "canonical_document_changed": false,
    ]
    return try makePrepared(
      capabilityID: "story.authoring.task.prepare", inputJSON: inputJSON,
      ownerOperation: "authoring.prepare", ownerInput: ownerInput,
      targetID: "\(projectID)/authoring/tasks/\(taskID)",
      targetRevision: String(projectRevision), preview: preview)
  }

  private func prepareCandidateSubmission(
    _ input: [String: Any], inputJSON: String, operationID: UUID
  ) async throws -> Prepared {
    guard exactKeys(input, required: ["task_id", "draft"], optional: ["based_on"]),
      let taskID = validID(input["task_id"]),
      let draft = input["draft"] as? [String: Any], validDraft(draft)
    else { throw ControlPlaneError.invalidRegistration("Invalid candidate submission input.") }
    let basedOn = input["based_on"] == nil ? nil : validID(input["based_on"])
    guard input["based_on"] == nil || basedOn != nil else {
      throw ControlPlaneError.invalidRegistration("Invalid based_on candidate ID.")
    }

    let status = try await ownerRead("authoring.status", input: [:], operationID: operationID)
    try ensurePreparing(operationID)
    guard let projectID = validID(status["project_id"]) else {
      throw ControlPlaneError.providerUnavailable("StoryProduction returned no project identity.")
    }
    let task = try await ownerRead(
      "authoring.task", input: ["id": taskID], operationID: operationID,
      expectedProjectID: projectID)
    try ensurePreparing(operationID)
    guard let taskValue = task["task"] as? [String: Any], taskValue["id"] as? String == taskID else {
      throw ControlPlaneError.stalePlan("The selected StoryProduction task changed.")
    }

    let candidateID = "semi-candidate-\(operationID.uuidString.lowercased())"
    let commandID = ownerCommandID("submit", operationID)
    var ownerInput: [String: Any] = [
      "command_id": commandID, "candidate_id": candidateID,
      "task_id": taskID, "draft": draft, "expected_project_id": projectID,
    ]
    if let basedOn { ownerInput["based_on"] = basedOn }
    let basedOnValue: Any = basedOn.map { $0 as Any } ?? NSNull()
    let preview: [String: Any] = [
      "action": "save_untrusted_story_candidate", "project_id": projectID,
      "task": [
        "id": taskID, "target": taskValue["target"] ?? NSNull(),
        "adoption_blockers": task["adoption_blockers"] ?? [],
      ],
      "candidate_id": candidateID, "based_on": basedOnValue, "draft": draft,
      "will_adopt": false,
    ]
    return try makePrepared(
      capabilityID: "story.authoring.candidate.submit", inputJSON: inputJSON,
      ownerOperation: "authoring.submit", ownerInput: ownerInput,
      targetID: "\(projectID)/tasks/\(taskID)/candidates/\(candidateID)",
      targetRevision: nil, preview: preview)
  }

  private func prepareCandidateReview(
    _ input: [String: Any], inputJSON: String, operationID: UUID
  ) async throws -> Prepared {
    guard exactKeys(
      input, required: ["candidate_id", "accept"], optional: ["note"]),
      let candidateID = validID(input["candidate_id"]),
      let accept = boolean(input["accept"])
    else { throw ControlPlaneError.invalidRegistration("Invalid candidate review input.") }
    let note = input["note"] as? String
    guard input["note"] == nil || (note != nil && (note?.utf8.count ?? 0) <= 4_096) else {
      throw ControlPlaneError.invalidRegistration("Invalid candidate review note.")
    }

    let status = try await ownerRead("authoring.status", input: [:], operationID: operationID)
    try ensurePreparing(operationID)
    guard let projectID = validID(status["project_id"]) else {
      throw ControlPlaneError.providerUnavailable("StoryProduction returned no project identity.")
    }
    let candidate = try await ownerRead(
      "authoring.candidate", input: ["id": candidateID], operationID: operationID,
      expectedProjectID: projectID)
    try ensurePreparing(operationID)
    guard let expectedHash = validHash(candidate["candidate_hash"]) else {
      throw ControlPlaneError.providerUnavailable("StoryProduction returned no candidate revision.")
    }
    guard let candidateData = candidate["candidate"] as? [String: Any],
      candidateData["id"] as? String == candidateID,
      validID(candidateData["task_id"]) != nil,
      let parent = candidateData["based_on"], parent is NSNull || validID(parent) != nil,
      candidateData["decision"] is NSNull
    else { throw ControlPlaneError.stalePlan("The candidate content or review state changed.") }

    guard let blockers = candidate["adoption_blockers"] as? [String],
      let draft = candidateData["draft"] as? [String: Any], validDraft(draft),
      let questions = draft["open_questions"] as? [String]
    else { throw ControlPlaneError.providerUnavailable("StoryProduction returned incomplete candidate review content.") }
    if accept, !blockers.isEmpty {
      throw ControlPlaneError.stalePlan("The candidate has adoption blockers; read its current task first.")
    }
    if accept, !questions.isEmpty {
      throw ControlPlaneError.stalePlan("The candidate has open questions that must be resolved before adoption.")
    }

    let commandID = ownerCommandID("review", operationID)
    var ownerInput: [String: Any] = [
      "command_id": commandID, "candidate_id": candidateID,
      "expected_candidate_hash": expectedHash, "accept": accept, "actor": "SEMI",
      "expected_project_id": projectID,
    ]
    if let note { ownerInput["note"] = note }
    let noteValue: Any = note.map { $0 as Any } ?? NSNull()
    let preview: [String: Any] = [
      "action": accept ? "adopt_story_candidate" : "reject_story_candidate",
      "project_id": projectID, "candidate": candidatePresentation(candidate),
      "decision": accept ? "accept" : "reject", "note": noteValue,
    ]
    return try makePrepared(
      capabilityID: "story.authoring.candidate.review", inputJSON: inputJSON,
      ownerOperation: "authoring.review", ownerInput: ownerInput,
      targetID: "\(projectID)/candidates/\(candidateID)", targetRevision: expectedHash,
      preview: preview)
  }

  private func candidatePresentation(_ candidate: [String: Any]) -> [String: Any] {
    var presentation = candidate
    presentation.removeValue(forKey: "candidate_hash")
    return presentation
  }

  private func makePrepared(
    capabilityID: String, inputJSON: String, ownerOperation: String,
    ownerInput: [String: Any], targetID: String, targetRevision: String?,
    preview: [String: Any]
  ) throws -> Prepared {
    guard let ownerCommandID = ownerInput["command_id"] as? String else {
      throw ControlPlaneError.invalidRegistration("StoryProduction owner command identity is missing.")
    }
    let ownerInputJSON = try encode(ownerInput)
    var reviewablePreview = preview
    reviewablePreview["owner_command_id"] = ownerCommandID
    let previewJSON = try encode(
      reviewablePreview, maximumBytes: ControlPlanePolicy.standard.maximumPreviewBytes)
    guard ownerInputJSON.utf8.count <= ControlPlanePolicy.standard.maximumPayloadBytes,
      previewJSON.utf8.count <= ControlPlanePolicy.standard.maximumPreviewBytes
    else { throw ControlPlaneError.invalidRegistration("StoryProduction plan exceeds the control-plane limit.") }
    let plan = AppCapabilityPlan(
      token: UUID().uuidString,
      target: AppEffectTarget(id: targetID, revision: targetRevision),
      previewJSON: previewJSON)
    return Prepared(
      capabilityID: capabilityID, inputJSON: inputJSON,
      ownerOperation: ownerOperation, ownerCommandID: ownerCommandID,
      ownerInputJSON: ownerInputJSON, plan: plan)
  }

  private func readRequest(capabilityID: String, inputJSON: String) throws -> (String, [String: Any]) {
    let input = try object(inputJSON)
    switch capabilityID {
    case "story.authoring.status":
      guard input.isEmpty else { throw StoryProviderError.invalidInput }
      return ("authoring.status", [:])
    case "story.authoring.document.read":
      guard exactKeys(input, required: ["kind"]),
        let kind = input["kind"] as? String,
        ["source", "story", "synopsis"].contains(kind)
      else { throw StoryProviderError.invalidInput }
      return ("authoring.document", ["kind": kind])
    case "story.authoring.candidate.read":
      guard exactKeys(input, required: ["id"]), let id = validID(input["id"]) else {
        throw StoryProviderError.invalidInput
      }
      return ("authoring.candidate", ["id": id])
    case "story.authoring.task.read":
      guard exactKeys(input, required: ["id"]), let id = validID(input["id"]) else {
        throw StoryProviderError.invalidInput
      }
      return ("authoring.task", ["id": id])
    default:
      throw ControlPlaneError.unknownCommand(capabilityID)
    }
  }

  private func ownerRead(
    _ operation: String, input: [String: Any], operationID: UUID,
    expectedProjectID: String? = nil
  ) async throws -> [String: Any] {
    var ownerInput = input
    if let expectedProjectID { ownerInput["expected_project_id"] = expectedProjectID }
    let json = try encode(ownerInput)
    let output = try await runProcess(operationID: operationID) {
      try await self.cli.read(operation: operation, inputJSON: json)
    }
    try ensureNotCancelled(operationID)
    let value = try StoryJSON.object(
      output, maximumBytes: ControlPlanePolicy.standard.maximumResultBytes)
    if let status = value["status"] as? String,
      ["not_applied", "not_started", "source_changed", "failed", "source_stale",
       "outcome_unknown", "uncertain", "cancelled"].contains(status)
    {
      throw ControlPlaneError.providerUnavailable("StoryProduction did not confirm the owner read.")
    }
    return value
  }

  private func runProcess<T: Sendable>(
    operationID: UUID, operation: @escaping @Sendable () async throws -> T
  ) async throws -> T {
    guard ownerProcesses.count < Self.maximumOwnerProcesses else {
      throw StoryProviderError.processCapacity
    }
    guard !Task.isCancelled, !cancelledOperations.contains(operationID),
      ownerProcesses[operationID] == nil
    else { throw CancellationError() }
    let task = Task { try await operation() }
    ownerProcesses[operationID] = OwnerProcess(
      cancel: { task.cancel() },
      drain: { _ = await task.result }
    )
    defer { ownerProcesses.removeValue(forKey: operationID) }
    return try await withTaskCancellationHandler {
      try await task.value
    } onCancel: {
      task.cancel()
    }
  }

  private func cancelAndDrainOwnerProcess(_ operationID: UUID) async {
    guard let process = ownerProcesses[operationID] else { return }
    process.cancel()
    await process.drain()
  }

  private func ensurePreparing(_ operationID: UUID) throws {
    try ensureNotCancelled(operationID)
    guard case .some(.preparing) = slots[operationID] else {
      throw CancellationError()
    }
  }

  private func ensureNotCancelled(_ operationID: UUID) throws {
    try Task.checkCancellation()
    guard !cancelledOperations.contains(operationID) else { throw CancellationError() }
  }

  private func reserve(_ operationID: UUID) throws {
    while slots.count >= Self.maximumPreparedOperations {
      guard !completedOrder.isEmpty else {
        throw ControlPlaneError.providerUnavailable("StoryProduction has too many live or unobserved operations.")
      }
      let oldest = completedOrder.removeFirst()
      if case .some(.completed(_, _, _)) = slots[oldest] { slots.removeValue(forKey: oldest) }
    }
    slots[operationID] = .preparing
  }

  private func finish(
    _ prepared: Prepared, key: String, result: AutomationResult, operationID: UUID
  ) {
    slots[operationID] = .completed(prepared, key, result)
    if !completedOrder.contains(operationID) { completedOrder.append(operationID) }
  }

  private func failed(_ code: String, operationID: UUID? = nil) throws -> AutomationResult {
    var value = ["status": "not_applied", "code": code]
    if let operationID { value["operation_id"] = operationID.uuidString }
    return try result(.failed, value)
  }

  private func uncertain(
    _ code: String, prepared: Prepared, operationID: UUID
  ) throws -> AutomationResult {
    try result(.uncertain, [
      "status": "outcome_unknown", "code": code,
      "operation_id": operationID.uuidString,
      "owner_command_id": prepared.ownerCommandID,
    ])
  }

  private func result(_ outcome: AutomationResult.Outcome, _ value: [String: String]) throws -> AutomationResult {
    let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    return AutomationResult(outcome: outcome, json: String(decoding: data, as: UTF8.self))
  }

  private func exactKeys(
    _ object: [String: Any], required: Set<String>, optional: Set<String> = []
  ) -> Bool {
    required.isSubset(of: Set(object.keys))
      && Set(object.keys).isSubset(of: required.union(optional))
  }

  private func object(_ inputJSON: String) throws -> [String: Any] {
    try StoryJSON.object(inputJSON, maximumBytes: ControlPlanePolicy.standard.maximumPayloadBytes)
  }

  private static func isNotAppliedResponse(_ output: String) -> Bool {
    guard let object = try? StoryJSON.object(
      output, maximumBytes: ControlPlanePolicy.standard.maximumResultBytes)
    else { return false }
    let status = object["status"] as? String
    return status == "not_applied" || status == "not_started"
  }

  private func encode(
    _ object: [String: Any], maximumBytes: Int = ControlPlanePolicy.standard.maximumPayloadBytes
  ) throws -> String {
    try StoryJSON.encode(object, maximumBytes: maximumBytes)
  }

  private func integer(_ value: Any?) -> UInt64? {
    (value as? StoryJSONNumber)?.unsignedInteger
  }

  private func validBrief(_ brief: [String: Any]) -> Bool {
    brief.allSatisfy { key, value in
      guard Self.briefFields.contains(key) else { return false }
      return ["preserve", "avoid"].contains(key) ? value is [String] : value is String
    }
  }

  private func validDraft(_ draft: [String: Any]) -> Bool {
    guard exactKeys(draft, required: ["blocks", "change_summary", "assumptions", "open_questions"]),
      let blocks = draft["blocks"] as? [[String: Any]], !blocks.isEmpty, blocks.count <= 512,
      let summary = draft["change_summary"] as? String, !summary.isEmpty,
      draft["assumptions"] is [String], draft["open_questions"] is [String]
    else { return false }
    var ids = Set<String>()
    return blocks.allSatisfy { block in
      guard exactKeys(block, required: ["id", "heading", "text"]),
        let id = validID(block["id"]), ids.insert(id).inserted,
        let heading = block["heading"] as? String, !heading.isEmpty,
        let text = block["text"] as? String, !text.isEmpty
      else { return false }
      return true
    }
  }

  private func confirmedMutation(_ value: [String: Any], prepared: Prepared) -> Bool {
    guard exactKeys(value, required: ["commit", "result"]),
      let input = try? object(prepared.ownerInputJSON),
      let projectID = validID(input["expected_project_id"]),
      let commit = value["commit"] as? [String: Any],
      exactKeys(commit, required: ["project_id", "revision", "event_count", "idempotent"]),
      commit["project_id"] as? String == projectID,
      let revision = integer(commit["revision"]), revision > 0,
      integer(commit["event_count"]) != nil, boolean(commit["idempotent"]) != nil,
      let result = value["result"] as? [String: Any]
    else { return false }
    switch prepared.ownerOperation {
    case "authoring.brief":
      guard let expectedBrief = input["brief"] as? [String: Any],
        let actualBrief = result["brief"] as? [String: Any], validBrief(actualBrief),
        expectedBrief.allSatisfy({ key, expected in
          guard let actual = actualBrief[key],
            let expectedJSON = try? encode(["value": expected]),
            let actualJSON = try? encode(["value": actual])
          else { return false }
          return expectedJSON == actualJSON
        })
      else { return false }
      return result["project_id"] as? String == projectID
        && integer(result["project_revision"]) == revision
        && integer(result["brief_revision"]) != nil
        && result["documents"] is [[String: Any]]
    case "authoring.prepare":
      guard let task = result["task"] as? [String: Any] else { return false }
      return task["id"] as? String == input["task_id"] as? String
        && task["target"] as? String == input["target"] as? String
        && task["instruction"] as? String == input["instruction"] as? String
        && boolean(task["cancelled"]) == false
        && task["selection"] as? [String] == input["selection"] as? [String]
        && result["adoption_blockers"] is [String]
    case "authoring.submit", "authoring.review":
      guard let candidate = result["candidate"] as? [String: Any],
        candidate["id"] as? String == input["candidate_id"] as? String,
        validID(candidate["task_id"]) != nil,
        let draft = candidate["draft"] as? [String: Any], validDraft(draft),
        validHash(result["candidate_hash"]) != nil,
        result["adoption_blockers"] is [String]
      else { return false }
      if prepared.ownerOperation == "authoring.submit" {
        guard let expectedDraft = input["draft"] as? [String: Any],
          let expectedJSON = try? encode(expectedDraft), let actualJSON = try? encode(draft)
        else { return false }
        let expectedParent = input["based_on"] as? String
        return candidate["decision"] is NSNull
          && candidate["task_id"] as? String == input["task_id"] as? String
          && actualJSON == expectedJSON
          && (candidate["based_on"] as? String == expectedParent)
          && (expectedParent != nil || candidate["based_on"] is NSNull)
      }
      guard let decision = candidate["decision"] as? [String: Any],
        let preview = try? StoryJSON.object(prepared.plan.previewJSON,
          maximumBytes: ControlPlanePolicy.standard.maximumPreviewBytes),
        let presented = preview["candidate"] as? [String: Any],
        let captured = presented["candidate"] as? [String: Any],
        candidate["task_id"] as? String == captured["task_id"] as? String,
        let capturedDraft = captured["draft"] as? [String: Any],
        let expectedDraftJSON = try? encode(capturedDraft), let actualDraftJSON = try? encode(draft),
        let capturedParent = captured["based_on"], let actualParent = candidate["based_on"],
        let expectedParentJSON = try? encode(["value": capturedParent]),
        let actualParentJSON = try? encode(["value": actualParent]),
        expectedDraftJSON == actualDraftJSON, expectedParentJSON == actualParentJSON
      else { return false }
      return boolean(decision["accepted"]) == boolean(input["accept"])
        && decision["actor"] as? String == "SEMI"
        && decision["candidate_hash"] as? String == input["expected_candidate_hash"] as? String
        && decision["note"] as? String == (input["note"] as? String ?? "")
        && result["candidate_hash"] as? String == input["expected_candidate_hash"] as? String
    default: return false
    }
  }

  private func boolean(_ value: Any?) -> Bool? {
    guard let number = value as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else { return nil }
    return number.boolValue
  }

  private func validID(_ value: Any?) -> String? {
    guard let value = value as? String, !value.isEmpty, value.utf8.count <= 64,
      value.unicodeScalars.allSatisfy({ scalar in
        switch scalar.value {
        case 45, 46, 48...57, 65...90, 95, 97...122: return true
        default: return false
        }
      })
    else { return nil }
    return value
  }

  private func validHash(_ value: Any?) -> String? {
    guard let value = value as? String, value.utf8.count == 64,
      value.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) })
    else { return nil }
    return value
  }

  private func ownerCommandID(_ action: String, _ operationID: UUID) -> String {
    "semi_\(action)_\(operationID.uuidString.lowercased())"
  }
}

private enum StoryProviderError: Error, Equatable { case invalidInput, processCapacity }
