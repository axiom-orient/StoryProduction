import Darwin
import Foundation
import SEMIControl

struct ProviderConfiguration: Sendable {
  let storyRoot: URL
  let workspace: URL
  let storyctl: URL
  let geultu: URL
  let instanceID: UUID
  private let workspaceAnchor: WorkspaceDirectoryAnchor

  static func parse(_ arguments: [String]) throws -> Self {
    var values: [String: String] = [:]
    var index = 0
    while index < arguments.count {
      let key = arguments[index]
      guard
        [
          "--story-root", "--workspace", "--storyctl-bin", "--geultu-bin", "--instance-id",
        ].contains(key),
        index + 1 < arguments.count,
        values[key] == nil
      else { throw ProviderConfigurationError.usage }
      values[key] = arguments[index + 1]
      index += 2
    }

    guard let storyRoot = absolutePath(values["--story-root"]),
      let workspace = absolutePath(values["--workspace"]),
      let geultu = absolutePath(values["--geultu-bin"]),
      let instanceID = values["--instance-id"].flatMap(UUID.init(uuidString:))
    else { throw ProviderConfigurationError.usage }

    let storyctl: URL
    if let selected = values["--storyctl-bin"] {
      guard let executable = absolutePath(selected) else {
        throw ProviderConfigurationError.usage
      }
      storyctl = executable
    } else {
      storyctl = storyRoot.appending(path: "integration/target/release/storyctl")
        .resolvingSymlinksInPath()
    }
    var storyRootIsDirectory: ObjCBool = false
    var workspaceIsDirectory: ObjCBool = false
    var storyctlIsDirectory: ObjCBool = false
    guard
      FileManager.default.fileExists(
        atPath: storyRoot.path, isDirectory: &storyRootIsDirectory),
      storyRootIsDirectory.boolValue,
      FileManager.default.fileExists(atPath: workspace.path, isDirectory: &workspaceIsDirectory),
      workspaceIsDirectory.boolValue,
      FileManager.default.fileExists(atPath: storyctl.path, isDirectory: &storyctlIsDirectory),
      !storyctlIsDirectory.boolValue
    else { throw ProviderConfigurationError.workspaceUnavailable }

    guard isRegularExecutable(storyctl), isRegularExecutable(geultu)
    else { throw ProviderConfigurationError.executableUnavailable }
    let workspaceAnchor = try WorkspaceDirectoryAnchor(path: workspace)

    return Self(
      storyRoot: storyRoot,
      workspace: workspace,
      storyctl: storyctl,
      geultu: geultu,
      instanceID: instanceID,
      workspaceAnchor: workspaceAnchor
    )
  }

  func validateWorkspace() throws {
    try workspaceAnchor.validate()
  }

  func addWorkspaceWorkingDirectoryAction(to actions: inout posix_spawn_file_actions_t?) throws {
    try validateWorkspace()
    try workspaceAnchor.addWorkingDirectoryAction(to: &actions)
  }

  var workspaceDescriptorPath: String { workspaceAnchor.childDescriptorPath }
  var workspaceChildDescriptor: Int32 { workspaceAnchor.childDescriptor }

  var displayName: String {
    let prefix = "StoryProduction · "
    var leaf = workspace.lastPathComponent
      .replacingOccurrences(of: "\n", with: " ")
      .replacingOccurrences(of: "\r", with: " ")
    while !leaf.isEmpty,
      (prefix + leaf).utf8.count > ControlPlanePolicy.standard.maximumDisplayNameBytes
    {
      leaf.removeLast()
    }
    return leaf.isEmpty ? "StoryProduction" : prefix + leaf
  }

  static let usage = """
    Usage: story-semi-provider \
      --story-root PATH --workspace PATH [--storyctl-bin PATH] --geultu-bin PATH --instance-id UUID

    SEMI supplies the instance ID when it starts the provider. The workspace and instance are fixed for that process.
    """

  private static func absolutePath(_ value: String?) -> URL? {
    guard let value, value.hasPrefix("/"), !value.contains("\0") else { return nil }
    // Freeze configured symlink aliases to the target selected for this process.
    return URL(fileURLWithPath: value).standardizedFileURL.resolvingSymlinksInPath()
  }

  private static func isRegularExecutable(_ path: URL) -> Bool {
    let attributes = try? FileManager.default.attributesOfItem(atPath: path.path)
    return attributes?[.type] as? FileAttributeType == .typeRegular
      && FileManager.default.isExecutableFile(atPath: path.path)
  }
}

enum ProviderConfigurationError: Error, LocalizedError {
  case usage
  case workspaceUnavailable
  case workspaceChanged
  case workspaceChangedDuringOwnerCall
  case workspaceSpawnActionFailed(Int32)
  case executableUnavailable

  var errorDescription: String? {
    switch self {
    case .usage: ProviderConfiguration.usage
    case .workspaceUnavailable: "The StoryProduction root or selected workspace is unavailable."
    case .workspaceChanged: "The selected StoryProduction workspace changed after provider startup."
    case .workspaceChangedDuringOwnerCall:
      "The selected StoryProduction workspace changed during an owner operation."
    case .workspaceSpawnActionFailed(let code):
      "The selected StoryProduction workspace could not be bound to the child process (posix_spawn error \(code))."
    case .executableUnavailable: "The selected storyctl or geultu executable is unavailable."
    }
  }
}

private final class WorkspaceDirectoryAnchor: Sendable {
  private let path: URL
  private let descriptor: Int32
  fileprivate let childDescriptor: Int32
  private let device: UInt64
  private let inode: UInt64

  init(path: URL) throws {
    // Hold the original directory open so its inode cannot be reused during this process.
    let descriptor = path.path.withCString {
      Darwin.open($0, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK)
    }
    guard descriptor >= 0 else { throw ProviderConfigurationError.workspaceUnavailable }

    var opened = stat()
    guard Darwin.fstat(descriptor, &opened) == 0,
      (opened.st_mode & mode_t(S_IFMT)) == mode_t(S_IFDIR)
    else {
      _ = Darwin.close(descriptor)
      throw ProviderConfigurationError.workspaceUnavailable
    }

    let childDescriptor = Darwin.fcntl(descriptor, F_DUPFD_CLOEXEC, 64)
    guard childDescriptor >= 0 else {
      let code = errno
      _ = Darwin.close(descriptor)
      throw ProviderConfigurationError.workspaceSpawnActionFailed(code)
    }

    self.path = path
    self.descriptor = descriptor
    self.childDescriptor = childDescriptor
    device = UInt64(truncatingIfNeeded: opened.st_dev)
    inode = UInt64(truncatingIfNeeded: opened.st_ino)
  }

  func validate() throws {
    var opened = stat()
    guard Darwin.fstat(descriptor, &opened) == 0,
      matches(opened.st_mode, device: opened.st_dev, inode: opened.st_ino)
    else { throw ProviderConfigurationError.workspaceChanged }

    var current = stat()
    let status = path.path.withCString {
      Darwin.fstatat(AT_FDCWD, $0, &current, AT_SYMLINK_NOFOLLOW)
    }
    guard status == 0,
      matches(current.st_mode, device: current.st_dev, inode: current.st_ino)
    else { throw ProviderConfigurationError.workspaceChanged }
  }

  func addWorkingDirectoryAction(to actions: inout posix_spawn_file_actions_t?) throws {
    let result = Darwin.posix_spawn_file_actions_addfchdir(&actions, descriptor)
    guard result == 0 else { throw ProviderConfigurationError.workspaceSpawnActionFailed(result) }
    let inheritanceResult = Darwin.posix_spawn_file_actions_adddup2(
      &actions, descriptor, childDescriptor)
    guard inheritanceResult == 0 else {
      throw ProviderConfigurationError.workspaceSpawnActionFailed(inheritanceResult)
    }
  }

  var childDescriptorPath: String { "/dev/fd/\(childDescriptor)" }

  deinit {
    _ = Darwin.close(childDescriptor)
    _ = Darwin.close(descriptor)
  }

  private func matches(_ mode: mode_t, device: dev_t, inode: ino_t) -> Bool {
    (mode & mode_t(S_IFMT)) == mode_t(S_IFDIR)
      && UInt64(truncatingIfNeeded: device) == self.device
      && UInt64(truncatingIfNeeded: inode) == self.inode
  }
}
