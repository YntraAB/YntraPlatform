import SwiftUI
import yntra_core
import CoreImage.CIFilterBuiltins

struct AuthView: View {
    @ObservedObject var viewModel: AuthViewModel
    
    @State private var email: String = ""
    @State private var pin: String = ""
    @State private var showCredentialsFallback: Bool = false
    
    // Brand Colors
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)
    private let primaryGradient = LinearGradient(
        colors: [Color(red: 0.31, green: 0.27, blue: 0.9), Color(red: 0.54, green: 0.36, blue: 0.96)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            // Glowing Ambient Bubbles
            VStack {
                Circle()
                    .fill(Color(red: 0.31, green: 0.27, blue: 0.9).opacity(0.15))
                    .frame(width: 300, height: 300)
                    .blur(radius: 50)
                    .offset(x: -80, y: -50)
                Spacer()
            }
            .ignoresSafeArea()

            VStack(spacing: 24) {
                Spacer()
                
                // Brand Logo/Header
                Image(systemName: "lock.shield.fill")
                    .font(.system(size: 64))
                    .foregroundStyle(primaryGradient)
                    .shadow(color: Color(red: 0.31, green: 0.27, blue: 0.9).opacity(0.4), radius: 10, y: 5)
                
                VStack(spacing: 8) {
                    Text("Yntra Secure Gate")
                        .font(.system(size: 28, weight: .black, design: .rounded))
                        .foregroundColor(.white)
                    
                    Text("Please authenticate to access your local workspace.")
                        .font(.system(size: 14, weight: .medium, design: .rounded))
                        .foregroundColor(.white.opacity(0.6))
                        .multilineTextAlignment(.center)
                        .padding(.horizontal, 32)
                }
                
                // Error banner
                if let errorMessage = viewModel.errorMessage {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.red)
                        Text(errorMessage)
                            .font(.system(size: 13, weight: .semibold, design: .rounded))
                            .foregroundColor(.white)
                    }
                    .padding()
                    .background(Color.red.opacity(0.2))
                    .cornerRadius(14)
                    .overlay(
                        RoundedRectangle(cornerRadius: 14)
                            .stroke(Color.red.opacity(0.3), lineWidth: 1)
                    )
                    .padding(.horizontal)
                }
                
                if viewModel.bankIdFlowState == "qr_scan" || viewModel.bankIdFlowState == "connecting" {
                    // BankID Scan Popup view
                    VStack(spacing: 20) {
                        Text(viewModel.bankIdFlowState == "connecting" ? "Connecting to Provider..." : "Scan QR Code")
                            .font(.system(size: 18, weight: .bold, design: .rounded))
                            .foregroundColor(.white)
                        
                        if viewModel.bankIdFlowState == "qr_scan", let qrData = viewModel.qrData {
                            // QR code box
                            ZStack {
                                RoundedRectangle(cornerRadius: 20)
                                    .fill(Color.white)
                                    .frame(width: 200, height: 200)
                                    .shadow(radius: 10)
                                
                                Image(uiImage: generateQRCode(from: qrData))
                                    .interpolation(.none)
                                    .resizable()
                                    .scaledToFit()
                                    .frame(width: 180, height: 180)
                            }
                        } else {
                            ProgressView()
                                .tint(.white)
                        }
                        
                        Button(action: {
                            viewModel.logout()
                        }) {
                            Text("Cancel")
                                .bold()
                                .padding(.horizontal, 24)
                                .padding(.vertical, 12)
                                .background(Color.red)
                                .foregroundColor(.white)
                                .cornerRadius(12)
                        }
                    }
                    .padding(30)
                    .background(cardBackground)
                    .cornerRadius(24)
                    .overlay(
                        RoundedRectangle(cornerRadius: 24)
                            .stroke(Color.white.opacity(0.1), lineWidth: 1)
                    )
                    .padding(.horizontal)
                } else {
                    // Selection view
                    VStack(spacing: 16) {
                        // Region Selector
                        HStack(spacing: 8) {
                            ForEach([("sv", "SE"), ("da", "DK"), ("no", "NO"), ("en", "US")], id: \.0) { code, label in
                                Button(action: { viewModel.setRegion(code) }) {
                                    Text(label)
                                        .font(.system(size: 14, weight: .bold, design: .rounded))
                                        .foregroundColor(.white)
                                        .frame(maxWidth: .infinity)
                                        .padding(.vertical, 12)
                                        .background(viewModel.authRegion == code ? Color.blue : cardBackground)
                                        .cornerRadius(12)
                                }
                            }
                        }
                        .padding(.horizontal)
                        
                        // Action buttons
                        Button(action: {
                            let provider = viewModel.authRegion == "sv" ? "se_bankid" : (viewModel.authRegion == "da" ? "dk_mitid" : "no_bankid")
                            viewModel.initiateBankIdLogin(provider: provider)
                        }) {
                            Text(viewModel.authRegion == "sv" ? "Login with Mobilt BankID" : (viewModel.authRegion == "da" ? "Login with MitID" : "Login with BankID"))
                                .font(.system(size: 16, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 16)
                                .background(primaryGradient)
                                .cornerRadius(16)
                                .shadow(color: Color(red: 0.31, green: 0.27, blue: 0.9).opacity(0.4), radius: 8, y: 4)
                        }
                        .padding(.horizontal)
                        
                        Button(action: {
                            viewModel.initiateBankIdLogin(provider: "card_or_badge")
                        }) {
                            Text("Authenticate via SITHS / NFC Badge")
                                .font(.system(size: 14, weight: .semibold, design: .rounded))
                                .foregroundColor(.white)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 14)
                                .background(cardBackground)
                                .cornerRadius(16)
                        }
                        .padding(.horizontal)
                        
                        // Credentials fallback
                        Button(action: {
                            withAnimation {
                                showCredentialsFallback.toggle()
                            }
                        }) {
                            Text(showCredentialsFallback ? "Hide Credentials Fallback" : "Use Credentials Fallback")
                                .font(.system(size: 13, weight: .bold, design: .rounded))
                                .foregroundColor(.purple)
                        }
                        .padding(.top, 8)
                        
                        if showCredentialsFallback {
                            VStack(spacing: 12) {
                                TextField("Email / Username", text: $email)
                                    .textFieldStyle(PlainTextFieldStyle())
                                    .padding()
                                    .background(cardBackground)
                                    .cornerRadius(14)
                                    .foregroundColor(.white)
                                
                                SecureField("PIN / Password", text: $pin)
                                    .textFieldStyle(PlainTextFieldStyle())
                                    .padding()
                                    .background(cardBackground)
                                    .cornerRadius(14)
                                    .foregroundColor(.white)
                                
                                Button(action: {
                                    viewModel.passwordLogin(email: email, pin: pin)
                                }) {
                                    Text("Submit Credentials")
                                        .bold()
                                        .foregroundColor(.white)
                                        .frame(maxWidth: .infinity)
                                        .padding()
                                        .background(Color.purple)
                                        .cornerRadius(14)
                                }
                            }
                            .padding(.horizontal)
                            .transition(.move(edge: .bottom).combined(with: .opacity))
                        }
                    }
                }
                
                Spacer()
            }
        }
    }

    private func generateQRCode(from string: String) -> UIImage {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)

        if let outputImage = filter.outputImage {
            let transform = CGAffineTransform(scaleX: 10, scaleY: 10)
            let scaledImage = outputImage.transformed(by: transform)
            if let cgImage = context.createCGImage(scaledImage, from: scaledImage.extent) {
                return UIImage(cgImage: cgImage)
            }
        }

        return UIImage(systemName: "xmark.circle") ?? UIImage()
    }
}
