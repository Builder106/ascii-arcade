import Vapor

public func makeApp() throws -> Application {
	var env = try Environment.detect()
	try LoggingSystem.bootstrap(from: &env)
	let app = Application(env)
	try configure(app)
	try routes(app)
	return app
}
