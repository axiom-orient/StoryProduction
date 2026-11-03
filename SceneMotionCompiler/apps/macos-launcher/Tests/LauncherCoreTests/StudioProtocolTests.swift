import Testing
@testable import LauncherCore

@Test func parsesReadyAndFailureProtocolLines() {
    #expect(parseStudioProtocolLine("SMC_STUDIO_READY\t{\"origin\":\"http://127.0.0.1:4321\",\"projectRoot\":\"/tmp/project\"}") == .ready(
        StudioReady(origin: "http://127.0.0.1:4321", projectRoot: "/tmp/project")
    ))
    #expect(parseStudioProtocolLine("SMC_STUDIO_ERROR\t{\"code\":\"project-not-ready\",\"message\":\"not ready\"}") == .failure(
        StudioFailure(code: "project-not-ready", message: "not ready")
    ))
}

@Test func ignoresMalformedOrUnrelatedOutput() {
    #expect(parseStudioProtocolLine("ordinary log output") == .ignored)
    #expect(parseStudioProtocolLine("SMC_STUDIO_READY\t{}") == .ignored)
    #expect(parseStudioProtocolLine("SMC_STUDIO_ERROR\tnot-json") == .ignored)
}
