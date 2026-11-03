import Darwin
import Foundation
import Testing

@testable import StoryProductionSEMIProvider

@Test func startupRejectsNonRegularExecutablesAndNeverFallsBackFromExplicitSelection() throws {
  let root = FileManager.default.temporaryDirectory.appending(
    path: "story-config-\(UUID().uuidString)")
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  defer { try? FileManager.default.removeItem(at: root) }
  let fallback = root.appending(path: "integration/target/release/storyctl")
  try FileManager.default.createDirectory(
    at: fallback.deletingLastPathComponent(), withIntermediateDirectories: true)
  try FileManager.default.createSymbolicLink(
    at: fallback, withDestinationURL: URL(fileURLWithPath: "/usr/bin/true"))
  let fifo = root.appending(path: "executable-fifo")
  try #require(mkfifo(fifo.path, mode_t(0o700)) == 0)
  let base = [
    "--story-root", root.path, "--workspace", root.path, "--instance-id", UUID().uuidString,
  ]
  for path in [fifo.path, root.path, root.appending(path: "missing").path, "relative/storyctl"] {
    #expect(throws: (any Error).self) {
      try ProviderConfiguration.parse(
        base + ["--storyctl-bin", path, "--geultu-bin", "/usr/bin/true"])
    }
    #expect(throws: (any Error).self) {
      try ProviderConfiguration.parse(
        base + ["--storyctl-bin", "/usr/bin/true", "--geultu-bin", path])
    }
  }
  #expect(
    try ProviderConfiguration.parse(base + ["--geultu-bin", "/usr/bin/true"]).storyctl.path
      == "/usr/bin/true")
  #expect(throws: ProviderConfigurationError.self) {
    try ProviderConfiguration.parse(
      base + ["--python", "/usr/bin/true", "--geultu-bin", "/usr/bin/true"])
  }
}

@Test func startupWorkspaceAnchorRejectsDirectoryReplacementWithoutOpeningTheNewWorkspace() throws {
  let root = FileManager.default.temporaryDirectory.appending(
    path: "story-config-\(UUID().uuidString)")
  let workspace = root.appending(path: "workspace")
  try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true)
  defer { try? FileManager.default.removeItem(at: root) }
  let configuration = try ProviderConfiguration.parse([
    "--story-root", root.path, "--workspace", workspace.path, "--instance-id", UUID().uuidString,
    "--storyctl-bin", "/usr/bin/true", "--geultu-bin", "/usr/bin/true",
  ])
  try FileManager.default.moveItem(at: workspace, to: root.appending(path: "selected-original"))
  try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true)
  #expect(throws: ProviderConfigurationError.self) {
    try configuration.validateWorkspace()
  }
}
