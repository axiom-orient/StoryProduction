import AppKit
import Foundation
import LauncherCore

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate, @unchecked Sendable {
    private let titleLabel = NSTextField(labelWithString: "SceneMotionCompiler Studio")
    private let projectLabel = NSTextField(wrappingLabelWithString: "프로젝트를 선택하세요.")
    private let statusLabel = NSTextField(wrappingLabelWithString: "SceneAssetCompiler에서 승인·게시한 프로젝트를 엽니다.")
    private let chooseButton = NSButton(title: "프로젝트 선택…", target: nil, action: nil)
    private let openButton = NSButton(title: "Studio 열기", target: nil, action: nil)
    private var window: NSWindow!
    private var studioProcess: Process?
    private var outputPipe: Pipe?
    private var outputBuffer = ""
    private var studioURL: URL?
    private var expectedProjectRoot: URL?
    private var expectedTermination = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        configureApplicationMenu()
        buildWindow()
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
        chooseProject(nil)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        expectedTermination = true
        outputPipe?.fileHandleForReading.readabilityHandler = nil
        if studioProcess?.isRunning == true { studioProcess?.terminate() }
    }

    @objc private func chooseProject(_ sender: Any?) {
        let panel = NSOpenPanel()
        panel.title = "SceneMotionCompiler 프로젝트 열기"
        panel.message = "SceneAssetCompiler에서 장면 이미지와 자산을 승인·게시한 프로젝트 폴더를 선택하세요."
        panel.prompt = "프로젝트 열기"
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let folder = panel.url else { return }
            self?.prepareAndStart(folder)
        }
    }

    @objc private func openStudio(_ sender: Any?) {
        guard let studioURL else { return }
        NSWorkspace.shared.open(studioURL)
    }

    private func prepareAndStart(_ folder: URL) {
        guard validateProject(folder) else { return }
        stopStudio()
        projectLabel.stringValue = folder.lastPathComponent
        statusLabel.stringValue = "Studio를 시작하는 중입니다…"
        chooseButton.isEnabled = false
        openButton.isEnabled = false
        studioURL = nil
        startStudio(for: folder)
    }

    private func validateProject(_ folder: URL) -> Bool {
        let handoff = folder.appendingPathComponent("handoff/scene-motion-asset-handoff.v2.json")
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: handoff.path, isDirectory: &isDirectory), !isDirectory.boolValue else {
            showAlert(
                title: "아직 열 수 없는 프로젝트입니다",
                message: "SceneAssetCompiler에서 이미지와 자산을 승인·게시한 뒤 다시 선택하세요."
            )
            return false
        }
        return true
    }

    private func startStudio(for project: URL) {
        do {
            expectedProjectRoot = project.standardizedFileURL.resolvingSymlinksInPath()
            let repositoryRoot = try locateRepositoryRoot()
            let script = repositoryRoot.appendingPathComponent("scripts/launch-studio-macos.mjs")
            guard let nodeURL = bundledNodeURL() else {
                throw NSError(domain: "SceneMotionCompilerLauncher", code: 2, userInfo: [
                    NSLocalizedDescriptionKey: "Node.js 실행 파일을 찾을 수 없습니다. 런처를 다시 빌드하세요."
                ])
            }

            let process = Process()
            let pipe = Pipe()
            process.executableURL = nodeURL
            process.arguments = [
                script.path,
                "--project", project.path,
                "--parent-pid", String(ProcessInfo.processInfo.processIdentifier)
            ]
            process.currentDirectoryURL = repositoryRoot
            process.standardOutput = pipe
            process.standardError = pipe
            process.terminationHandler = { [weak self] terminated in
                DispatchQueue.main.async { self?.processDidTerminate(terminated) }
            }
            pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else { return }
                DispatchQueue.main.async { self?.consumeOutput(text) }
            }
            studioProcess = process
            outputPipe = pipe
            expectedTermination = false
            try process.run()
        } catch {
            restoreIdleState(message: "Studio를 시작하지 못했습니다.")
            showAlert(title: "Studio를 시작할 수 없습니다", message: error.localizedDescription)
        }
    }

    private func consumeOutput(_ text: String) {
        outputBuffer += text
        while let newline = outputBuffer.firstIndex(of: "\n") {
            let line = String(outputBuffer[..<newline])
            outputBuffer.removeSubrange(...newline)
            switch parseStudioProtocolLine(line) {
            case .ready(let ready):
                handleReady(ready)
            case .failure(let failure):
                handleFailure(failure)
            case .ignored:
                break
            }
        }
    }

    private func handleReady(_ ready: StudioReady) {
        let reportedProjectRoot: URL? = {
            guard ready.projectRoot.hasPrefix("/"), !ready.projectRoot.contains("\0") else { return nil }
            return URL(fileURLWithPath: ready.projectRoot)
                .standardizedFileURL
                .resolvingSymlinksInPath()
        }()
        guard let reportedProjectRoot,
              let expectedProjectRoot,
              reportedProjectRoot.path == expectedProjectRoot.path,
              let url = URL(string: ready.origin),
              url.scheme == "http",
              url.host == "127.0.0.1",
              let port = url.port,
              (1...65_535).contains(port),
              url.user == nil,
              url.password == nil,
              url.query == nil,
              url.fragment == nil,
              url.path.isEmpty || url.path == "/" else {
            stopStudio()
            restoreIdleState(message: "Studio 시작 응답의 프로젝트 또는 로컬 주소를 확인하지 못했습니다.")
            return
        }
        studioURL = url
        statusLabel.stringValue = "준비되었습니다. 작업 화면을 열었습니다."
        chooseButton.isEnabled = true
        openButton.isEnabled = true
        NSWorkspace.shared.open(url)
    }

    private func handleFailure(_ failure: StudioFailure) {
        let message = failure.message
        stopStudio()
        restoreIdleState(message: message)
        showAlert(title: "프로젝트를 열 수 없습니다", message: message)
    }

    private func processDidTerminate(_ process: Process) {
        guard studioProcess === process else { return }
        outputPipe?.fileHandleForReading.readabilityHandler = nil
        outputPipe = nil
        studioProcess = nil
        let wasReady = studioURL != nil
        guard !expectedTermination else { return }
        studioURL = nil
        let message = wasReady
            ? "Studio가 종료되었습니다. 프로젝트를 다시 선택하세요."
            : "Studio가 준비되기 전에 종료되었습니다. 프로젝트를 다시 선택하세요."
        restoreIdleState(message: message)
        showAlert(title: "Studio를 계속 실행할 수 없습니다", message: message)
    }

    private func stopStudio() {
        expectedTermination = true
        outputPipe?.fileHandleForReading.readabilityHandler = nil
        if studioProcess?.isRunning == true { studioProcess?.terminate() }
        studioProcess = nil
        outputPipe = nil
        studioURL = nil
        expectedProjectRoot = nil
        outputBuffer = ""
    }

    private func restoreIdleState(message: String) {
        expectedProjectRoot = nil
        statusLabel.stringValue = message
        chooseButton.isEnabled = true
        openButton.isEnabled = false
    }

    private func bundledNodeURL() -> URL? {
        guard let resourceURL = Bundle.main.resourceURL else { return nil }
        let pathFile = resourceURL.appendingPathComponent("node-path.txt")
        guard let path = try? String(contentsOf: pathFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines),
              !path.isEmpty,
              FileManager.default.isExecutableFile(atPath: path) else { return nil }
        return URL(fileURLWithPath: path)
    }

    private func locateRepositoryRoot() throws -> URL {
        let appParent = Bundle.main.bundleURL.deletingLastPathComponent()
        let candidates = [appParent, appParent.deletingLastPathComponent()]
        for candidate in candidates {
            let script = candidate.appendingPathComponent("scripts/launch-studio-macos.mjs")
            if FileManager.default.fileExists(atPath: script.path) { return candidate }
        }
        throw NSError(domain: "SceneMotionCompilerLauncher", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "런처와 SceneMotionCompiler 저장소의 위치가 맞지 않습니다. 앱을 저장소 루트에 두세요."
        ])
    }

    private func showAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = title
        alert.informativeText = message
        alert.addButton(withTitle: "확인")
        alert.beginSheetModal(for: window)
    }

    private func buildWindow() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 520, height: 250),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "SceneMotionCompiler Studio"
        window.center()
        window.delegate = self

        titleLabel.font = .systemFont(ofSize: 24, weight: .semibold)
        projectLabel.font = .systemFont(ofSize: 16, weight: .medium)
        projectLabel.textColor = .labelColor
        statusLabel.textColor = .secondaryLabelColor
        chooseButton.target = self
        chooseButton.action = #selector(chooseProject(_:))
        chooseButton.bezelStyle = .rounded
        openButton.target = self
        openButton.action = #selector(openStudio(_:))
        openButton.bezelStyle = .rounded
        openButton.keyEquivalent = "\r"
        openButton.isEnabled = false

        let buttons = NSStackView(views: [chooseButton, openButton])
        buttons.orientation = .horizontal
        buttons.spacing = 10
        let content = NSStackView(views: [titleLabel, projectLabel, statusLabel, buttons])
        content.orientation = .vertical
        content.alignment = .leading
        content.spacing = 14
        content.edgeInsets = NSEdgeInsets(top: 28, left: 30, bottom: 28, right: 30)
        content.translatesAutoresizingMaskIntoConstraints = false
        window.contentView?.addSubview(content)
        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: window.contentView!.leadingAnchor),
            content.trailingAnchor.constraint(equalTo: window.contentView!.trailingAnchor),
            content.topAnchor.constraint(equalTo: window.contentView!.topAnchor),
            content.bottomAnchor.constraint(lessThanOrEqualTo: window.contentView!.bottomAnchor),
            statusLabel.widthAnchor.constraint(equalToConstant: 450)
        ])
    }

    private func configureApplicationMenu() {
        let mainMenu = NSMenu()
        let appItem = NSMenuItem()
        mainMenu.addItem(appItem)
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "SceneMotionCompiler Studio 종료", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = appMenu
        NSApp.mainMenu = mainMenu
    }
}

MainActor.assumeIsolated {
    let app = NSApplication.shared
    let delegate = AppDelegate()
    app.delegate = delegate
    app.run()
}
