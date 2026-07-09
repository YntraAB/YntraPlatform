import SwiftUI
import yntra_core

struct DirectoryView: View {
    @ObservedObject var viewModel: DirectoryViewModel
    
    @State private var activeTab: String = "users" // "users" | "teams"
    @State private var showInviteSheet: Bool = false
    @State private var showTeamSheet: Bool = false
    
    @State private var inviteName: String = ""
    @State private var inviteEmail: String = ""
    @State private var inviteRole: String = "assistant"
    
    @State private var newTeamName: String = ""

    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(spacing: 20) {
                // Header
                HStack {
                    Text("Directory")
                        .font(.system(size: 28, weight: .black, design: .rounded))
                        .foregroundColor(.white)
                    Spacer()
                    Button(action: {
                        if activeTab == "users" {
                            showInviteSheet = true
                        } else {
                            showTeamSheet = true
                        }
                    }) {
                        Image(systemName: "plus")
                            .font(.system(size: 20, weight: .bold))
                            .foregroundColor(.white)
                            .frame(width: 44, height: 44)
                            .background(Color.blue)
                            .cornerRadius(12)
                    }
                }
                .padding(.horizontal)
                .padding(.top, 10)
                
                // Tabs
                HStack(spacing: 8) {
                    Button(action: { activeTab = "users" }) {
                        Text("Staff Directory")
                            .font(.system(size: 13, weight: .bold, design: .rounded))
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 10)
                            .background(activeTab == "users" ? Color.blue : cardBackground)
                            .cornerRadius(10)
                    }
                    Button(action: { activeTab = "teams" }) {
                        Text("Workspace Teams")
                            .font(.system(size: 13, weight: .bold, design: .rounded))
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 10)
                            .background(activeTab == "teams" ? Color.blue : cardBackground)
                            .cornerRadius(10)
                    }
                }
                .padding(.horizontal)
                
                if let errorMessage = viewModel.errorMessage {
                    Text("Error: \(errorMessage)")
                        .foregroundColor(.red)
                        .font(.caption)
                }

                // List
                ScrollView {
                    VStack(spacing: 12) {
                        if activeTab == "users" {
                            ForEach(viewModel.users, id: \.id) { user in
                                HStack(spacing: 12) {
                                    Image(systemName: "person.crop.circle.fill")
                                        .font(.system(size: 36))
                                        .foregroundColor(.white.opacity(0.5))
                                    
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text(user.fullName ?? "Unnamed User")
                                            .font(.system(size: 15, weight: .bold, design: .rounded))
                                            .foregroundColor(.white)
                                        Text("\(user.role.uppercased()) • \(user.email)")
                                            .font(.system(size: 11, weight: .medium, design: .rounded))
                                            .foregroundColor(.white.opacity(0.5))
                                    }
                                    
                                    Spacer()
                                    
                                    Button(action: { viewModel.deleteWorkspaceUser(userId: user.id) }) {
                                        Image(systemName: "trash")
                                            .foregroundColor(.red)
                                    }
                                }
                                .padding(16)
                                .background(cardBackground)
                                .cornerRadius(16)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 16)
                                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                )
                            }
                        } else {
                            ForEach(viewModel.teams, id: \.id) { team in
                                HStack {
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text(team.name)
                                            .font(.system(size: 16, weight: .bold, design: .rounded))
                                            .foregroundColor(.white)
                                        Text("Workspace Scoped Team")
                                            .font(.system(size: 12, weight: .medium, design: .rounded))
                                            .foregroundColor(.white.opacity(0.5))
                                    }
                                    Spacer()
                                }
                                .padding(18)
                                .background(cardBackground)
                                .cornerRadius(16)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 16)
                                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                )
                            }
                        }
                    }
                    .padding(.horizontal)
                }
            }
        }
        .sheet(isPresented: $showInviteSheet) {
            NavigationView {
                Form {
                    Section(header: Text("Staff Details")) {
                        TextField("Full Name", text: $inviteName)
                        TextField("Email", text: $inviteEmail)
                        TextField("Role", text: $inviteRole)
                    }
                }
                .navigationTitle("Invite Staff")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Cancel") { showInviteSheet = false }
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button("Invite") {
                            viewModel.inviteUser(email: inviteEmail, name: inviteName, role: inviteRole)
                            showInviteSheet = false
                            inviteName = ""
                            inviteEmail = ""
                            inviteRole = "assistant"
                        }
                        .disabled(inviteName.isEmpty || inviteEmail.isEmpty)
                    }
                }
            }
        }
        .sheet(isPresented: $showTeamSheet) {
            NavigationView {
                Form {
                    Section(header: Text("Team Details")) {
                        TextField("Team Name", text: $newTeamName)
                    }
                }
                .navigationTitle("Create Workspace Team")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Cancel") { showTeamSheet = false }
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button("Create") {
                            viewModel.createTeam(name: newTeamName)
                            showTeamSheet = false
                            newTeamName = ""
                        }
                        .disabled(newTeamName.isEmpty)
                    }
                }
            }
        }
    }
}

// Conform FFI entities to Identifiable
extension WorkspaceUser: Identifiable {}
extension Team: Identifiable {}
extension TeamEvent: Identifiable {}
