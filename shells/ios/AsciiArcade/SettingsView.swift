import SwiftUI

struct SettingsView: View {
    @AppStorage("sceneId")   private var sceneId:   String = "matrix"
    @AppStorage("themeName") private var themeName: String = "Hacker"

    private let scenes: [String] = AaEngine.sceneNames()
    private let themes: [String] = Theme.all.map(\.name)

    var body: some View {
        List {
            Section("Scene") {
                ForEach(scenes, id: \.self) { (id: String) in
                    Button {
                        sceneId = id
                    } label: {
                        HStack {
                            Text(id.capitalized)
                            Spacer()
                            if id == sceneId {
                                Image(systemName: "checkmark")
                                    .foregroundStyle(.tint)
                            }
                        }
                    }
                    .foregroundStyle(.primary)
                }
            }

            Section("Theme") {
                ForEach(themes, id: \.self) { (name: String) in
                    Button {
                        themeName = name
                    } label: {
                        HStack {
                            Circle()
                                .fill(Color(Theme.named(name).text))
                                .frame(width: 12, height: 12)
                            Text(name)
                            Spacer()
                            if name == themeName {
                                Image(systemName: "checkmark")
                                    .foregroundStyle(.tint)
                            }
                        }
                    }
                    .foregroundStyle(.primary)
                }
            }
        }
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.inline)
    }
}
