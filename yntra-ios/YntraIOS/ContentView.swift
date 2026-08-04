import SwiftUI
import yntra_core

struct ContentView: View {
    @StateObject private var authViewModel = AuthViewModel()
    @StateObject private var dashboardViewModel = DashboardViewModel()
    @StateObject private var messagingViewModel = MessagingViewModel()
    @StateObject private var directoryViewModel = DirectoryViewModel()
    @StateObject private var settingsViewModel = SettingsViewModel()

    @StateObject private var careViewModel = CareViewModel()
    @StateObject private var jobsViewModel = JobsViewModel()
    @State private var pendingSyncCount: UInt32 = 0

    private var activeModules: [String: Bool] {
        guard let jsonStr = settingsViewModel.workspace?.modulesActive,
              let data = jsonStr.data(using: .utf8),
              let dict = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] else {
            return [:]
        }
        return dict.compactMapValues { $0 as? Bool }
    }
    
    private var isMessagingActive: Bool {
        settingsViewModel.workspace == nil || activeModules["messaging"] == true
    }
    
    private var isDirectoryActive: Bool {
        settingsViewModel.workspace == nil || activeModules["directory"] == true
    }

    private var isAssistanceActive: Bool {
        settingsViewModel.workspace == nil || activeModules["assistance"] == true
    }

    private var isJobsActive: Bool {
        settingsViewModel.workspace == nil || activeModules["jobs"] == true
    }

    var body: some View {
        if !authViewModel.isLoggedIn {
            AuthView(viewModel: authViewModel)
        } else {
            VStack(spacing: 0) {
                HStack {
                    Text("Yntra")
                        .font(.headline)
                        .fontWeight(.bold)
                        .foregroundColor(.white)

                    Spacer()

                    HStack(spacing: 4) {
                        Circle()
                            .fill(pendingSyncCount == 0 ? Color.green : Color.orange)
                            .frame(width: 8, height: 8)

                        Text(pendingSyncCount == 0 ? "Synced" : "Pending (\(pendingSyncCount))")
                            .font(.caption)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(Color(red: 0.12, green: 0.16, blue: 0.23))
                    .cornerRadius(12)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
                .background(Color(red: 0.06, green: 0.09, blue: 0.16))

                TabView {
                    DashboardView(viewModel: dashboardViewModel)
                        .tabItem {
                            Label("Home", systemImage: "house.fill")
                        }
                    
                    if isMessagingActive {
                        MessagingView(viewModel: messagingViewModel)
                            .tabItem {
                                Label("Messages", systemImage: "envelope.fill")
                            }
                    }
                    
                    if isAssistanceActive {
                        CareView(viewModel: careViewModel)
                            .tabItem {
                                Label("Care", systemImage: "heart.text.square.fill")
                            }
                    }
                    
                    if isJobsActive {
                        JobsView(viewModel: jobsViewModel)
                            .tabItem {
                                Label("Jobs", systemImage: "shippingbox.fill")
                            }
                    }
                    
                    if isDirectoryActive {
                        DirectoryView(viewModel: directoryViewModel)
                            .tabItem {
                                Label("Directory", systemImage: "person.2.fill")
                            }
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

                    Task {
                        self.pendingSyncCount = (try? await getMobileSyncQueueSummary(workspaceId: "workspace-1")) ?? 0
                    }
                }
            }
        }
    }
}
