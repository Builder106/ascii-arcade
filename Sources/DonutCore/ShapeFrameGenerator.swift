public protocol ShapeFrameGenerator {
    var width: Int { get }
    var height: Int { get }
    func frame(atTime t: Double) -> String
}
