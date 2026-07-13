import UIKit

struct Theme: Equatable {
    let name: String
    let text: UIColor
    let background: UIColor

    static let hacker = Theme(
        name: "Hacker",
        text: UIColor(red: 48/255, green: 209/255, blue: 88/255, alpha: 1),
        background: .black
    )
    static let amber = Theme(
        name: "Amber",
        text: UIColor(red: 1, green: 166/255, blue: 0, alpha: 1),
        background: UIColor(red: 26/255, green: 8/255, blue: 0, alpha: 1)
    )
    static let ice = Theme(
        name: "Ice",
        text: UIColor(red: 0, green: 1, blue: 1, alpha: 1),
        background: UIColor(red: 0, green: 13/255, blue: 26/255, alpha: 1)
    )
    static let ghost = Theme(
        name: "Ghost",
        text: UIColor(red: 28/255, green: 28/255, blue: 30/255, alpha: 1),
        background: UIColor(red: 245/255, green: 245/255, blue: 245/255, alpha: 1)
    )

    static let all: [Theme] = [.hacker, .amber, .ice, .ghost]

    static func named(_ name: String) -> Theme {
        all.first { $0.name.lowercased() == name.lowercased() } ?? .hacker
    }
}
