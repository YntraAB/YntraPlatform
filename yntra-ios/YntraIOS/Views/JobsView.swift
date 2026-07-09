import SwiftUI
import yntra_core

struct JobsView: View {
    @ObservedObject var viewModel: JobsViewModel
    
    @State private var selectedJob: JobTicket? = nil
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(alignment: .leading, spacing: 20) {
                // Header
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Work Orders")
                            .font(.system(size: 28, weight: .black, design: .rounded))
                            .foregroundColor(.white)
                        Text("Logistics & Move Management")
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
                
                if viewModel.jobs.isEmpty {
                    VStack(spacing: 12) {
                        Spacer()
                        Image(systemName: "shippingbox.fill")
                            .font(.system(size: 40))
                            .foregroundColor(.white.opacity(0.3))
                        Text("No job tickets assigned")
                            .font(.system(size: 14, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.5))
                        Spacer()
                    }
                    .frame(maxWidth: .infinity)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 12) {
                            ForEach(viewModel.jobs, id: \.id) { job in
                                Button(action: {
                                    selectedJob = job
                                }) {
                                    VStack(alignment: .leading, spacing: 10) {
                                        HStack {
                                            Text(job.title)
                                                .font(.system(size: 16, weight: .bold, design: .rounded))
                                                .foregroundColor(.white)
                                                .multilineTextAlignment(.leading)
                                            
                                            Spacer()
                                            
                                            StatusBadgeView(status: job.status)
                                        }
                                        
                                        Text(job.description)
                                            .font(.system(size: 13, weight: .medium, design: .rounded))
                                            .foregroundColor(.white.opacity(0.6))
                                            .lineLimit(2)
                                            .multilineTextAlignment(.leading)
                                        
                                        Divider()
                                            .background(Color.white.opacity(0.1))
                                        
                                        HStack {
                                            HStack(spacing: 6) {
                                                Image(systemName: "mappin.and.ellipse")
                                                    .font(.system(size: 14))
                                                    .foregroundColor(Color(red: 0.55, green: 0.36, blue: 0.96))
                                                Text(job.locationAddress)
                                                    .font(.system(size: 12, weight: .medium, design: .rounded))
                                                    .foregroundColor(.white.opacity(0.8))
                                            }
                                            
                                            Spacer()
                                            
                                            Text(job.scheduledDate)
                                                .font(.system(size: 12, weight: .bold, design: .rounded))
                                                .foregroundColor(Color(red: 0.55, green: 0.36, blue: 0.96))
                                        }
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
        .sheet(item: $selectedJob) { job in
            JobDetailView(job: job, viewModel: viewModel)
        }
    }
}

struct StatusBadgeView: View {
    let status: String
    
    private var badgeColors: (bg: Color, text: Color) {
        switch status {
        case "assigned":
            return (Color(red: 0.99, green: 0.95, blue: 0.78), Color(red: 0.85, green: 0.47, blue: 0.02))
        case "in_progress":
            return (Color(red: 0.86, green: 0.92, blue: 1.0), Color(red: 0.15, green: 0.39, blue: 0.92))
        case "completed":
            return (Color(red: 0.82, green: 0.98, blue: 0.9), Color(red: 0.02, green: 0.59, blue: 0.41))
        default:
            return (Color(red: 0.95, green: 0.96, blue: 0.96), Color(red: 0.29, green: 0.33, blue: 0.39))
        }
    }
    
    var body: some View {
        Text(status.replacingOccurrences(of: "_", with: " ").uppercased())
            .font(.system(size: 10, weight: .black, design: .rounded))
            .foregroundColor(badgeColors.text)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(badgeColors.bg)
            .cornerRadius(8)
    }
}

struct JobDetailView: View {
    let job: JobTicket
    @ObservedObject var viewModel: JobsViewModel
    @Environment(\.presentationMode) var presentationMode
    
    @State private var checklist: [ChecklistTaskItem] = []
    @State private var reportText: String = ""
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)
    
    var body: some View {
        NavigationView {
            ZStack {
                darkBackground.ignoresSafeArea()
                
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(job.title)
                                .font(.title3)
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                            
                            HStack {
                                StatusBadgeView(status: job.status)
                                Spacer()
                                Text(job.scheduledDate)
                                    .font(.caption)
                                    .foregroundColor(.white.opacity(0.6))
                            }
                        }
                        
                        Text(job.description)
                            .font(.system(size: 14, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.8))
                        
                        Divider()
                            .background(Color.white.opacity(0.1))
                        
                        // Route Details
                        if job.originAddress != nil || job.destinationAddress != nil {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("ROUTE DETAILS")
                                    .font(.system(size: 11, weight: .bold))
                                    .foregroundColor(.white.opacity(0.5))
                                
                                if let origin = job.originAddress {
                                    Text("Origin: \(origin) (Floor: \(job.originFloor), Elevator: \(job.originHasElevator ? "Yes" : "No"))")
                                        .font(.system(size: 13, weight: .medium, design: .rounded))
                                        .foregroundColor(.white)
                                }
                                
                                if let dest = job.destinationAddress {
                                    Text("Destination: \(dest) (Floor: \(job.destinationFloor), Elevator: \(job.destinationHasElevator ? "Yes" : "No"))")
                                        .font(.system(size: 13, weight: .medium, design: .rounded))
                                        .foregroundColor(.white)
                                }
                            }
                            
                            Divider()
                                .background(Color.white.opacity(0.1))
                        }
                        
                        // Checklist Section
                        VStack(alignment: .leading, spacing: 10) {
                            Text("CHECKLIST / TASKS")
                                .font(.system(size: 11, weight: .bold))
                                .foregroundColor(.white.opacity(0.5))
                            
                            if checklist.isEmpty {
                                Text("No checklist items.")
                                    .font(.system(size: 13, weight: .medium, design: .rounded))
                                    .foregroundColor(.white.opacity(0.5))
                            } else {
                                ForEach(0..<checklist.count, id: \.self) { index in
                                    HStack(spacing: 12) {
                                        Button(action: {
                                            if job.status == "in_progress" {
                                                checklist[index].done.toggle()
                                            }
                                        }) {
                                            Image(systemName: checklist[index].done ? "checkmark.square.fill" : "square")
                                                .font(.system(size: 20))
                                                .foregroundColor(checklist[index].done ? Color(red: 0.55, green: 0.36, blue: 0.96) : .white.opacity(0.5))
                                        }
                                        .disabled(job.status != "in_progress")
                                        
                                        Text(checklist[index].text)
                                            .font(.system(size: 13, weight: .medium, design: .rounded))
                                            .foregroundColor(checklist[index].done ? .white.opacity(0.5) : .white)
                                        
                                        Spacer()
                                    }
                                }
                            }
                        }
                        
                        // Completion Report
                        if job.status == "in_progress" {
                            Divider()
                                .background(Color.white.opacity(0.1))
                            
                            VStack(alignment: .leading, spacing: 8) {
                                Text("SUBMIT COMPLETION REPORT")
                                    .font(.system(size: 11, weight: .bold))
                                    .foregroundColor(.white.opacity(0.5))
                                
                                TextField("Describe the completed work...", text: $reportText)
                                    .textFieldStyle(PlainTextFieldStyle())
                                    .padding(12)
                                    .background(cardBackground)
                                    .cornerRadius(10)
                                    .foregroundColor(.white)
                                    .overlay(
                                        RoundedRectangle(cornerRadius: 10)
                                            .stroke(Color.white.opacity(0.1), lineWidth: 1)
                                    )
                            }
                        } else if job.status == "completed" {
                            Divider()
                                .background(Color.white.opacity(0.1))
                            
                            VStack(alignment: .leading, spacing: 8) {
                                Text("COMPLETION REPORT")
                                    .font(.system(size: 11, weight: .bold))
                                    .foregroundColor(.white.opacity(0.5))
                                
                                Text(job.completionReport ?? "No report text submitted.")
                                    .font(.system(size: 13, weight: .medium, design: .rounded))
                                    .foregroundColor(.white.opacity(0.8))
                            }
                        }
                        
                        Spacer(minLength: 40)
                        
                        // Action Buttons
                        if job.status == "assigned" {
                            Button(action: {
                                viewModel.updateStatus(jobId: job.id, status: "in_progress")
                                presentationMode.wrappedValue.dismiss()
                            }) {
                                Text("Start Job")
                                    .foregroundColor(.white)
                                    .frame(maxWidth: .infinity)
                                    .padding()
                                    .background(Color(red: 0.55, green: 0.36, blue: 0.96))
                                    .cornerRadius(12)
                            }
                        } else if job.status == "in_progress" {
                            Button(action: {
                                let encoder = JSONEncoder()
                                if let data = try? encoder.encode(checklist),
                                   let jsonString = String(data: data, encoding: .utf8) {
                                    viewModel.completeJob(jobId: job.id, checklistJson: jsonString, completionReport: reportText)
                                }
                                presentationMode.wrappedValue.dismiss()
                            }) {
                                Text("Complete & Submit")
                                    .foregroundColor(.white)
                                    .frame(maxWidth: .infinity)
                                    .padding()
                                    .background(Color(red: 0.1, green: 0.73, blue: 0.51))
                                    .cornerRadius(12)
                            }
                        }
                    }
                    .padding()
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(trailing: Button("Close") {
                presentationMode.wrappedValue.dismiss()
            }.foregroundColor(.white))
            .onAppear {
                parseChecklist()
            }
        }
    }
    
    private func parseChecklist() {
        guard let data = job.checklistJson.data(using: .utf8) else { return }
        let decoder = JSONDecoder()
        if let decoded = try? decoder.decode([ChecklistTaskItem].self, from: data) {
            self.checklist = decoded
        }
    }
}

struct ChecklistTaskItem: Codable, Identifiable {
    var id = UUID()
    let text: String
    var done: Bool
    
    enum CodingKeys: String, CodingKey {
        case text, done
    }
}

extension JobTicket: Identifiable {}
