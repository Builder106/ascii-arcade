import Vapor

public func configure(_ app: Application) throws {
	// Serve files from Public directory
	app.middleware.use(FileMiddleware(publicDirectory: app.directory.publicDirectory))
	let env = Environment.get("DOOM_PORT") ?? Environment.get("PORT")
	if let s = env, let p = Int(s) {
		app.http.server.configuration.port = p
	} else {
		app.http.server.configuration.port = 8787
	}
}
