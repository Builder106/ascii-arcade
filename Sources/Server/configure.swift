import Vapor

public func configure(_ app: Application) throws {
	// Serve files from Public directory
	app.middleware.use(FileMiddleware(publicDirectory: app.directory.publicDirectory))
	app.http.server.configuration.port = 8787
}
