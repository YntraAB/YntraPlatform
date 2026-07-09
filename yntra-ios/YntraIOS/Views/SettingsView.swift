import SwiftUI
import yntra_core

struct SettingsView: View {
    @ObservedObject var viewModel: SettingsViewModel
    var onLogout: () -> Void

    @State private var name: String = "Operator 1"
    @State private var phone: String = "+46 70 123 45 67"
    @State private var preferredLanguage: String = "sv"

    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(spacing: 20) {
                // Header
                HStack {
                    Text("User Settings")
                        .font(.system(size: 28, weight: .black, design: .rounded))
                        .foregroundColor(.white)
                    Spacer()
                }
                .padding(.horizontal)
                .padding(.top, 10)

                if let errorMessage = viewModel.errorMessage {
                    Text("Error: \(errorMessage)")
                        .foregroundColor(.red)
                        .font(.caption)
                        .padding(.horizontal)
                }

                // Profile form card
                VStack(alignment: .leading, spacing: 16) {
                    Text("Edit Profile Info")
                        .font(.system(size: 16, weight: .bold, design: .rounded))
                        .foregroundColor(.white)
                    
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Display Name")
                            .font(.caption)
                            .foregroundColor(.white.opacity(0.5))
                        TextField("", text: $name)
                            .padding()
                            .background(darkBackground)
                            .cornerRadius(12)
                            .foregroundColor(.white)
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Phone Number")
                            .font(.caption)
                            .foregroundColor(.white.opacity(0.5))
                        TextField("", text: $phone)
                            .padding()
                            .background(darkBackground)
                            .cornerRadius(12)
                            .foregroundColor(.white)
                    }

                    // Select Language
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Preferred Language")
                            .font(.caption)
                            .foregroundColor(.white.opacity(0.5))
                        
                        HStack(spacing: 6) {
                            ForEach([("sv", "SV"), ("da", "DA"), ("no", "NO"), ("fi", "FI"), ("en", "EN")], id: \.0) { code, label in
                                Button(action: { preferredLanguage = code }) {
                                    Text(label)
                                        .font(.system(size: 11, weight: .bold, design: .rounded))
                                        .foregroundColor(.white)
                                        .frame(maxWidth: .infinity)
                                        .padding(.vertical, 8)
                                        .background(preferredLanguage == code ? Color.purple : Color.white.opacity(0.1))
                                        .cornerRadius(8)
                                }
                            }
                        }
                    }

                    Button(action: {
                        viewModel.updateProfile(fullName: name, phone: phone, languagePreference: preferredLanguage)
                    }) {
                        Text("Save Profile Settings")
                            .bold()
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.blue)
                            .cornerRadius(12)
                    }
                }
                .padding(20)
                .background(cardBackground)
                .cornerRadius(20)
                .overlay(
                    RoundedRectangle(cornerRadius: 20)
                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                )
                .padding(.horizontal)

                // Workspace info panel
                VStack(alignment: .leading, spacing: 8) {
                    Text("Active Workspace Configuration")
                        .font(.system(size: 15, weight: .bold, design: .rounded))
                        .foregroundColor(.white)
                    
                    if let ws = viewModel.workspace {
                        Text("Name: \(ws.name)")
                            .font(.system(size: 13, weight: .semibold, design: .rounded))
                            .foregroundColor(.white.opacity(0.8))
                        Text("Active Modules: \(ws.modulesActive)")
                            .font(.system(size: 11, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.5))
                    } else {
                        Text("Loading config...")
                            .font(.caption)
                            .foregroundColor(.white.opacity(0.5))
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(20)
                .background(cardBackground)
                .cornerRadius(20)
                .overlay(
                    RoundedRectangle(cornerRadius: 20)
                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                )
                .padding(.horizontal)

                Spacer()

                // Logout button
                Button(action: onLogout) {
                    HStack(spacing: 8) {
                        Image(systemName: "power")
                        Text("Log Out Session")
                            .bold()
                    }
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.red)
                    .cornerRadius(16)
                }
                .padding(.horizontal)
                .padding(.bottom, 10)
            }
        }
    }
}
