import SwiftUI
import yntra_core

struct DashboardView: View {
    @ObservedObject var viewModel: DashboardViewModel
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)

    private var activeModules: [String: Bool] {
        guard let jsonStr = viewModel.workspace?.modulesActive,
              let data = jsonStr.data(using: .utf8),
              let dict = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] else {
            return [:]
        }
        return dict.compactMapValues { $0 as? Bool }
    }
    
    private var isTodosActive: Bool {
        viewModel.workspace == nil || activeModules["todos"] == true
    }
    
    private var isSchedulingActive: Bool {
        viewModel.workspace == nil || activeModules["scheduling"] == true
    }

    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            // Glowing Ambient Bubbles
            VStack {
                HStack {
                    Circle()
                        .fill(Color(red: 0.31, green: 0.27, blue: 0.9).opacity(0.12))
                        .frame(width: 250, height: 250)
                        .blur(radius: 50)
                        .offset(x: -80, y: -50)
                    Spacer()
                }
                Spacer()
            }
            .ignoresSafeArea()

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    // Header
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Workspace Home")
                                .font(.system(size: 28, weight: .black, design: .rounded))
                                .foregroundColor(.white)
                            Text("Welcome back, Operator")
                                .font(.system(size: 13, weight: .medium, design: .rounded))
                                .foregroundColor(.white.opacity(0.6))
                        }
                        Spacer()
                    }
                    .padding(.horizontal)
                    .padding(.top, 10)
                    
                    if isTodosActive || isSchedulingActive {
                        // Stats grid
                        VStack(alignment: .leading, spacing: 16) {
                            Text("Active Modules Overview")
                                .font(.system(size: 16, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                            
                            HStack(spacing: 16) {
                                if isTodosActive {
                                    // Tasks widget
                                    VStack(alignment: .leading, spacing: 6) {
                                        Text("Tasks Done")
                                            .font(.caption)
                                            .foregroundColor(.white.opacity(0.5))
                                        Text("\(viewModel.completedTodosCount)/\(viewModel.todosCount)")
                                            .font(.system(size: 22, weight: .black, design: .rounded))
                                            .foregroundColor(.white)
                                    }
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .padding()
                                    .background(cardBackground)
                                    .cornerRadius(16)
                                    .overlay(
                                        RoundedRectangle(cornerRadius: 16)
                                            .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                    )
                                }
                                
                                if isSchedulingActive {
                                    // Events widget
                                    VStack(alignment: .leading, spacing: 6) {
                                        Text("Active Events")
                                            .font(.caption)
                                            .foregroundColor(.white.opacity(0.5))
                                        Text("\(viewModel.events.count)")
                                            .font(.system(size: 22, weight: .black, design: .rounded))
                                            .foregroundColor(.white)
                                    }
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .padding()
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

                    if let errorMessage = viewModel.errorMessage {
                        Text("Error: \(errorMessage)")
                            .font(.caption)
                            .foregroundColor(.red)
                            .padding(.horizontal)
                    }

                    if isSchedulingActive {
                        // Schedule Events List
                        VStack(alignment: .leading, spacing: 12) {
                            Text("Upcoming Schedule / Timetable")
                                .font(.system(size: 16, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                            
                            if viewModel.events.isEmpty {
                                HStack {
                                    Spacer()
                                    VStack(spacing: 8) {
                                        Image(systemName: "calendar.badge.exclamationmark")
                                            .font(.system(size: 32))
                                            .foregroundColor(.white.opacity(0.3))
                                        Text("No upcoming events scheduled")
                                            .font(.system(size: 14, weight: .medium, design: .rounded))
                                            .foregroundColor(.white.opacity(0.5))
                                    }
                                    .padding(.vertical, 40)
                                    Spacer()
                                }
                                .background(cardBackground)
                                .cornerRadius(20)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 20)
                                        .stroke(Color.white.opacity(0.05), lineWidth: 1)
                                )
                            } else {
                                ForEach(viewModel.events, id: \.id) { event in
                                    HStack {
                                        VStack(alignment: .leading, spacing: 4) {
                                            Text(event.title)
                                                .font(.system(size: 15, weight: .bold, design: .rounded))
                                                .foregroundColor(.white)
                                            Text("\(event.startTime) - \(event.endTime)")
                                                .font(.system(size: 12, weight: .medium, design: .rounded))
                                                .foregroundColor(.white.opacity(0.5))
                                        }
                                        Spacer()
                                        Image(systemName: "info.circle")
                                            .foregroundColor(Color.blue)
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
                .padding(.bottom, 20)
            }
        }
    }
}
