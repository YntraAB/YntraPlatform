import SwiftUI
import yntra_core

struct CareView: View {
    @ObservedObject var viewModel: CareViewModel
    
    @State private var selectedClient: ClientProfile? = nil
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(alignment: .leading, spacing: 20) {
                // Header
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Care Assistance")
                            .font(.system(size: 28, weight: .black, design: .rounded))
                            .foregroundColor(.white)
                        Text("Client Care & Health Records")
                            .font(.system(size: 13, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.6))
                    }
                    Spacer()
                }
                .padding(.horizontal)
                .padding(.top, 10)
                
                if let errorMessage = viewModel.errorMessage {
                    Text("Error: \(errorMessage)")
                        .font(.caption)
                        .foregroundColor(.red)
                        .padding(.horizontal)
                }
                
                if viewModel.clients.isEmpty {
                    VStack(spacing: 12) {
                        Spacer()
                        Image(systemName: "heart.text.square.fill")
                            .font(.system(size: 40))
                            .foregroundColor(.white.opacity(0.3))
                        Text("No care assistance clients registered")
                            .font(.system(size: 14, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.5))
                        Spacer()
                    }
                    .frame(maxWidth: .infinity)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 12) {
                            ForEach(viewModel.clients, id: \.id) { client in
                                Button(action: {
                                    selectedClient = client
                                    viewModel.loadHealthRecords(clientId: client.id)
                                }) {
                                    HStack {
                                        VStack(alignment: .leading, spacing: 6) {
                                            Text("\(client.firstName) \(client.lastName)")
                                                .font(.system(size: 16, weight: .bold, design: .rounded))
                                                .foregroundColor(.white)
                                            
                                            Text("Personal ID: \(client.personalNumber ?? "Redacted")")
                                                .font(.system(size: 12, weight: .medium, design: .rounded))
                                                .foregroundColor(.white.opacity(0.5))
                                        }
                                        
                                        Spacer()
                                        
                                        Text(client.careLevel ?? "STANDARD CARE")
                                            .font(.system(size: 10, weight: .black, design: .rounded))
                                            .foregroundColor(Color(red: 0.55, green: 0.36, blue: 0.96))
                                            .padding(.horizontal, 8)
                                            .padding(.vertical, 4)
                                            .background(Color(red: 0.55, green: 0.36, blue: 0.96).opacity(0.15))
                                            .cornerRadius(8)
                                    }
                                    .padding(18)
                                    .background(cardBackground)
                                    .cornerRadius(18)
                                    .overlay(
                                        RoundedRectangle(cornerRadius: 18)
                                            .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                    )
                                }
                            }
                        }
                        .padding(.horizontal)
                    }
                }
            }
        }
        .sheet(item: $selectedClient) { client in
            ClientDetailView(client: client, viewModel: viewModel)
        }
    }
}

struct ClientDetailView: View {
    let client: ClientProfile
    @ObservedObject var viewModel: CareViewModel
    @Environment(\.presentationMode) var presentationMode
    
    @State private var activeTab = 0 // 0 -> Journals, 1 -> Medications
    @State private var newJournalText = ""
    @State private var showAddMedSheet = false
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)
    
    var body: some View {
        NavigationView {
            ZStack {
                darkBackground.ignoresSafeArea()
                
                VStack(spacing: 16) {
                    // Custom Profile Header
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("\(client.firstName) \(client.lastName)")
                                .font(.title3)
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                            Text("Care Level: \(client.careLevel ?? "Standard")")
                                .font(.caption)
                                .foregroundColor(.white.opacity(0.6))
                        }
                        Spacer()
                    }
                    .padding(.horizontal)
                    .padding(.top)
                    
                    Picker("Segment", selection: $activeTab) {
                        Text("Daily Journal").tag(0)
                        Text("Medications").tag(1)
                    }
                    .pickerStyle(SegmentedPickerStyle())
                    .padding(.horizontal)
                    
                    if activeTab == 0 {
                        // Daily Journal List
                        if viewModel.journals.isEmpty {
                            VStack(spacing: 8) {
                                Spacer()
                                Text("No care journal records found.")
                                    .foregroundColor(.white.opacity(0.5))
                                    .font(.subheadline)
                                Spacer()
                            }
                        } else {
                            ScrollView {
                                LazyVStack(spacing: 10) {
                                    ForEach(viewModel.journals, id: \.id) { entry in
                                        VStack(alignment: .leading, spacing: 6) {
                                            Text(entry.content)
                                                .font(.system(size: 14, weight: .medium, design: .rounded))
                                                .foregroundColor(.white)
                                            
                                            HStack {
                                                Text("By: \(entry.authorId ?? "Assistant")")
                                                Spacer()
                                                Text(entry.createdAt)
                                            }
                                            .font(.system(size: 10))
                                            .foregroundColor(.white.opacity(0.4))
                                        }
                                        .padding(14)
                                        .background(cardBackground)
                                        .cornerRadius(12)
                                        .overlay(
                                            RoundedRectangle(cornerRadius: 12)
                                                .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                        )
                                    }
                                }
                                .padding(.horizontal)
                            }
                        }
                        
                        // Journal Input Bar
                        HStack {
                            TextField("Log daily care progress...", text: $newJournalText)
                                .textFieldStyle(PlainTextFieldStyle())
                                .padding(12)
                                .background(cardBackground)
                                .cornerRadius(10)
                                .foregroundColor(.white)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 10)
                                        .stroke(Color.white.opacity(0.1), lineWidth: 1)
                                )
                            
                            Button(action: {
                                if !newJournalText.trimmingCharacters(in: .whitespaces).isEmpty {
                                    viewModel.addJournal(clientId: client.id, content: newJournalText)
                                    newJournalText = ""
                                }
                            }) {
                                Image(systemName: "paperplane.fill")
                                    .foregroundColor(.white)
                                    .padding(12)
                                    .background(Color(red: 0.55, green: 0.36, blue: 0.96))
                                    .cornerRadius(10)
                            }
                        }
                        .padding()
                        
                    } else {
                        // Medications List
                        if viewModel.medications.isEmpty {
                            VStack(spacing: 8) {
                                Spacer()
                                Text("No medications logged.")
                                    .foregroundColor(.white.opacity(0.5))
                                    .font(.subheadline)
                                Spacer()
                            }
                        } else {
                            ScrollView {
                                LazyVStack(spacing: 10) {
                                    ForEach(viewModel.medications, id: \.id) { med in
                                        VStack(alignment: .leading, spacing: 6) {
                                            Text(med.name)
                                                .font(.system(size: 15, weight: .bold, design: .rounded))
                                                .foregroundColor(.white)
                                            
                                            Text("Dosage: \(med.dosage ?? "N/A") | Frequency: \(med.frequency ?? "N/A")")
                                                .font(.system(size: 12, weight: .medium, design: .rounded))
                                                .foregroundColor(.white.opacity(0.8))
                                            
                                            if let instructions = med.instructions {
                                                Text("Instructions: \(instructions)")
                                                    .font(.system(size: 11, weight: .regular, design: .rounded))
                                                    .foregroundColor(.white.opacity(0.5))
                                            }
                                        }
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .padding(14)
                                        .background(cardBackground)
                                        .cornerRadius(12)
                                        .overlay(
                                            RoundedRectangle(cornerRadius: 12)
                                                .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                        )
                                    }
                                }
                                .padding(.horizontal)
                            }
                        }
                        
                        Button(action: { showAddMedSheet = true }) {
                            HStack {
                                Image(systemName: "plus")
                                Text("Add Medication Plan")
                            }
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color(red: 0.55, green: 0.36, blue: 0.96))
                            .cornerRadius(12)
                        }
                        .padding()
                    }
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(trailing: Button("Close") {
                presentationMode.wrappedValue.dismiss()
            }.foregroundColor(.white))
            .sheet(isPresented: $showAddMedSheet) {
                AddMedicationView(onSubmit: { name, dosage, frequency, instructions in
                    viewModel.addMed(clientId: client.id, name: name, dosage: dosage, frequency: frequency, instructions: instructions)
                })
            }
        }
    }
}

struct AddMedicationView: View {
    @Environment(\.presentationMode) var presentationMode
    let onSubmit: (String, String, String, String) -> Void
    
    @State private var name = ""
    @State private var dosage = ""
    @State private var frequency = ""
    @State private var instructions = ""
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var body: some View {
        NavigationView {
            ZStack {
                darkBackground.ignoresSafeArea()
                
                Form {
                    Section(header: Text("Details").foregroundColor(.white.opacity(0.6))) {
                        TextField("Medication Name", text: $name)
                        TextField("Dosage (e.g. 50mg)", text: $dosage)
                        TextField("Frequency (e.g. Twice Daily)", text: $frequency)
                        TextField("Instructions", text: $instructions)
                    }
                    .listRowBackground(cardBackground)
                    .foregroundColor(.white)
                }
                .scrollContentBackground(.hidden)
            }
            .navigationTitle("Add Medication Plan")
            .navigationBarItems(
                leading: Button("Cancel") {
                    presentationMode.wrappedValue.dismiss()
                }.foregroundColor(.white),
                trailing: Button("Save") {
                    if !name.trimmingCharacters(in: .whitespaces).isEmpty {
                        onSubmit(name, dosage, frequency, instructions)
                        presentationMode.wrappedValue.dismiss()
                    }
                }.foregroundColor(.white)
            )
        }
    }
}

extension ClientProfile: Identifiable {}
extension JournalEntry: Identifiable {}
extension MedicationItem: Identifiable {}
