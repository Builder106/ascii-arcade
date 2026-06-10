import Vapor

let app = try makeApp()
defer { app.shutdown() }
try app.run()
