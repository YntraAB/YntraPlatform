import SwiftUI
import yntra_core

struct ContentView: View {
    @StateObject private var authViewModel = AuthViewModel()
    @StateObject private var dashboardViewModel = DashboardViewModel()
    @StateObject private var messagingViewModel = MessagingViewModel()
    @StateObject private var directoryViewModel = DirectoryViewModel()
    @StateObject private var settingsViewModel = SettingsViewModel()

    var body: some View {
        if !authViewModel.isLoggedIn {
            AuthView(viewModel: authViewModel)
        } else {
            TabView {
                DashboardView(viewModel: dashboardViewModel)
                    .tabItem {
                        Label("Home", systemImage: "house.fill")
                    }
                
                MessagingView(viewModel: messagingViewModel)
                    .tabItem {
                        Label("Messages", systemImage: "envelope.fill")
                    }
                
                DirectoryView(viewModel: directoryViewModel)
                    .tabItem {
                        Label("Directory", systemImage: "person.2.fill")
                    }
                
                SettingsView(viewModel: settingsViewModel, onLogout: {
                    authViewModel.logout()
                })
                .tabItem {
                    Label("Settings", systemImage: "gearshape.fill")
                }
            }
            .accentColor(.blue)
            .onAppear {
                // Configure SwiftUI Tab Bar dark styling
                let appearance = UITabBarAppearance()
                appearance.configureWithOpaqueBackground()
                appearance.backgroundColor = UIColor(red: 0.12, green: 0.16, blue: 0.23, alpha: 1.0)
                UITabBar.appearance().standardAppearance = appearance
                UITabBar.appearance().scrollEdgeAppearance = appearance
            }
        }
    }
}
