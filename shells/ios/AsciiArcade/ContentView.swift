import SwiftUI

struct ContentView: View {
    var body: some View {
        TabView {
            NavigationStack {
                SceneView()
                    .navigationBarHidden(true)
            }
            .tabItem {
                Label("Scene", systemImage: "play.rectangle.fill")
            }

            NavigationStack {
                SettingsView()
            }
            .tabItem {
                Label("Settings", systemImage: "gearshape.fill")
            }
        }
        .toolbarBackground(.ultraThinMaterial, for: .tabBar)
        .toolbarBackground(.visible, for: .tabBar)
    }
}
