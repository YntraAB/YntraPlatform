import SwiftUI
import yntra_core

struct MessagingView: View {
    @ObservedObject var viewModel: MessagingViewModel
    
    @State private var activeTab: String = "inbox" // "inbox" | "sent" | "trash"
    @State private var selectedMessage: MessageItem? = nil
    @State private var showComposeSheet: Bool = false
    
    @State private var composeSubject: String = ""
    @State private var composeBody: String = ""
    @State private var composeReceiverId: String = ""
    @State private var composeTeamId: String = ""
    @State private var isTeamTarget: Bool = false

    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var filteredMessages: [MessageItem] {
        if activeTab == "inbox" {
            return viewModel.messages.filter { $0.receiverId == "user-1" || $0.targetTeamId != nil }
        } else {
            return viewModel.messages.filter { $0.senderId == "user-1" }
        }
    }

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(spacing: 20) {
                // Header
                HStack {
                    Text("Communications")
                        .font(.system(size: 28, weight: .black, design: .rounded))
                        .foregroundColor(.white)
                    Spacer()
                    Button(action: { showComposeSheet = true }) {
                        Image(systemName: "square.and.pencil")
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
                    ForEach([("inbox", "Inbox"), ("sent", "Sent"), ("trash", "Trash")], id: \.0) { tabId, label in
                        Button(action: { activeTab = tabId }) {
                            Text(label)
                                .font(.system(size: 13, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 10)
                                .background(activeTab == tabId ? Color.blue : cardBackground)
                                .cornerRadius(10)
                        }
                    }
                }
                .padding(.horizontal)
                
                // Message List
                if filteredMessages.isEmpty {
                    VStack(spacing: 12) {
                        Spacer()
                        Image(systemName: "envelope.badge")
                            .font(.system(size: 40))
                            .foregroundColor(.white.opacity(0.3))
                        Text("No messages in \(activeTab)")
                            .font(.system(size: 14, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.5))
                        Spacer()
                    }
                } else {
                    ScrollView {
                        VStack(spacing: 10) {
                            ForEach(filteredMessages, id: \.id) { message in
                                VStack(alignment: .leading, spacing: 6) {
                                    HStack {
                                        Text(message.subject ?? "(No Subject)")
                                            .font(.system(size: 15, weight: message.isRead ? .medium : .bold, design: .rounded))
                                            .foregroundColor(.white)
                                        Spacer()
                                        if !message.isRead {
                                            Circle()
                                                .fill(Color.green)
                                                .frame(width: 8, height: 8)
                                        }
                                    }
                                    
                                    Text(message.body?.prefix(60) ?? "")
                                        .font(.system(size: 13, weight: .medium, design: .rounded))
                                        .foregroundColor(.white.opacity(0.6))
                                }
                                .padding(18)
                                .background(message.isRead ? cardBackground.opacity(0.6) : cardBackground)
                                .cornerRadius(16)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 16)
                                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                )
                                .onTapGesture {
                                    selectedMessage = message
                                    viewModel.markAsRead(messageId: message.id)
                                }
                            }
                        }
                        .padding(.horizontal)
                    }
                }
            }
        }
        .sheet(isPresented: $showComposeSheet) {
            NavigationView {
                Form {
                    Section(header: Text("Recipient Option")) {
                        Picker("Target", selection: $isTeamTarget) {
                            Text("Direct User").tag(false)
                            Text("Team Collective").tag(true)
                        }
                        .pickerStyle(SegmentedPickerStyle())
                        
                        if isTeamTarget {
                            TextField("Team ID", text: $composeTeamId)
                        } else {
                            TextField("Receiver User ID", text: $composeReceiverId)
                        }
                    }
                    
                    Section(header: Text("Content")) {
                        TextField("Subject", text: $composeSubject)
                        TextEditor(text: $composeBody)
                            .frame(height: 150)
                    }
                }
                .navigationTitle("Compose")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button("Cancel") { showComposeSheet = false }
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button("Send") {
                            if isTeamTarget {
                                viewModel.sendMessageToTeam(teamId: composeTeamId, subject: composeSubject, body: composeBody)
                            } else {
                                viewModel.sendMessageToUser(receiverId: composeReceiverId, subject: composeSubject, body: composeBody)
                            }
                            showComposeSheet = false
                            composeSubject = ""
                            composeBody = ""
                            composeReceiverId = ""
                            composeTeamId = ""
                        }
                        .disabled(composeSubject.isEmpty || composeBody.isEmpty)
                    }
                }
            }
        }
        .alert(item: $selectedMessage) { message in
            Alert(
                title: Text(message.subject ?? "(No Subject)"),
                message: Text("From: \(message.senderId ?? "System")\n\n\(message.body ?? "")"),
                dismissButton: .default(Text("Close"))
            )
        }
    }
}

// Make MessageItem conforms to Identifiable
extension MessageItem: Identifiable {}
