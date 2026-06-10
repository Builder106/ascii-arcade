import XCTest
import Vapor
@testable import Server

final class ServerTests: XCTestCase {
    /// `configure` + `routes` wire up cleanly and register the DOOM WebSocket route.
    func testRoutesRegisterDoomWebSocket() throws {
        let app = Application(.testing)
        defer { app.shutdown() }

        try configure(app)
        try routes(app)

        let hasDoomRoute = app.routes.all.contains { route in
            route.path.map(\.description).joined(separator: "/").contains("doom")
        }
        XCTAssertTrue(hasDoomRoute, "Expected a /ws/doom route to be registered")
    }
}
