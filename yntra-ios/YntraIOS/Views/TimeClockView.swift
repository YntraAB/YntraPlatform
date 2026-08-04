import SwiftUI
import yntra_core

struct TimeClockView: View {
    @StateObject private var locationManager = LocationManager()
    @State private var userId: String = "usr_field_01"
    @State private var workspaceId: String = "ws_default"
    @State private var hours: String = "8.0"
    @State private var shiftNote: String = "Field job shift"
    @State private var clockStatusMessage: String? = nil
    @State private var isClockedIn: Bool = false
    @State private var activeShiftStartTime: Date? = nil
    
    private let darkBackground = Color(red: 0.04, green: 0.06, blue: 0.1)
    private let cardBackground = Color(red: 0.12, green: 0.16, blue: 0.23)
    private let primaryPurple = Color(red: 0.55, green: 0.36, blue: 0.96)
    
    var body: some View {
        ZStack {
            darkBackground.ignoresSafeArea()
            
            VStack(alignment: .leading, spacing: 20) {
                // Header
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("GPS Time Clock")
                            .font(.system(size: 28, weight: .black, design: .rounded))
                            .foregroundColor(.white)
                        Text("Geofenced Field Shift Clocking")
                            .font(.system(size: 13, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.6))
                    }
                    Spacer()
                }
                .padding(.horizontal)
                .padding(.top, 10)
                
                // Geofence Badge & Location Card
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Image(systemName: locationManager.isWithinGeofence ? "checkmark.seal.fill" : "location.slash.fill")
                            .foregroundColor(locationManager.isWithinGeofence ? .green : .orange)
                            .font(.system(size: 20))
                        
                        VStack(alignment: .leading, spacing: 2) {
                            Text(locationManager.isWithinGeofence ? "Inside Job Site Geofence" : "Outside Geofence Radius")
                                .font(.system(size: 15, weight: .bold, design: .rounded))
                                .foregroundColor(.white)
                            
                            Text("Distance: \(String(format: "%.1f", locationManager.distanceFromTargetMeters))m from site (Limit: \(Int(locationManager.maxRadiusMeters))m)")
                                .font(.system(size: 12, weight: .medium, design: .rounded))
                                .foregroundColor(.white.opacity(0.6))
                        }
                        Spacer()
                    }
                    
                    if let userLoc = locationManager.userLocation {
                        Text("Coordinates: \(userLoc.coordinate.latitude), \(userLoc.coordinate.longitude)")
                            .font(.system(size: 11, weight: .monospaced))
                            .foregroundColor(.white.opacity(0.4))
                    }
                }
                .padding(16)
                .background(cardBackground)
                .cornerRadius(16)
                .overlay(
                    RoundedRectangle(cornerRadius: 16)
                        .stroke(locationManager.isWithinGeofence ? Color.green.opacity(0.3) : Color.orange.opacity(0.3), lineWidth: 1)
                )
                .padding(.horizontal)
                
                if let statusMsg = clockStatusMessage {
                    Text(statusMsg)
                        .font(.system(size: 13, weight: .medium, design: .rounded))
                        .foregroundColor(statusMsg.contains("Violation") || statusMsg.contains("Failed") ? .red : .green)
                        .padding(.horizontal)
                }
                
                // Clock Controls Card
                VStack(alignment: .leading, spacing: 16) {
                    Text("SHIFT DETAILS")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.white.opacity(0.5))
                    
                    TextField("Shift Notes...", text: $shiftNote)
                        .textFieldStyle(PlainTextFieldStyle())
                        .padding(12)
                        .background(darkBackground)
                        .cornerRadius(10)
                        .foregroundColor(.white)
                    
                    TextField("Logged Hours (e.g. 8.0)", text: $hours)
                        .textFieldStyle(PlainTextFieldStyle())
                        .padding(12)
                        .background(darkBackground)
                        .cornerRadius(10)
                        .foregroundColor(.white)
                    
                    Button(action: {
                        triggerClockIn()
                    }) {
                        HStack {
                            Image(systemName: isClockedIn ? "stop.circle.fill" : "play.circle.fill")
                                .font(.system(size: 22))
                            Text(isClockedIn ? "Clock Out & Submit Shift" : "GPS Clock In")
                                .font(.system(size: 16, weight: .bold, design: .rounded))
                        }
                        .foregroundColor(.white)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 16)
                        .background(isClockedIn ? Color.red : (locationManager.isWithinGeofence ? primaryPurple : Color.gray))
                        .cornerRadius(14)
                    }
                    .disabled(!locationManager.isWithinGeofence && !isClockedIn)
                }
                .padding(18)
                .background(cardBackground)
                .cornerRadius(18)
                .padding(.horizontal)
                
                Spacer()
            }
        }
        .onAppear {
            locationManager.requestLocationPermission()
        }
    }
    
    private func triggerClockIn() {
        guard let userLoc = locationManager.userLocation else {
            clockStatusMessage = "Failed: Device GPS location unavailable"
            return
        }
        
        let hrs = Double(hours) ?? 8.0
        let dateStr = DateFormatter.yyyyMMdd.string(from: Date())
        
        Task {
            do {
                if !isClockedIn {
                    let report = try await clockInGeofenced(
                        requesterUserId: userId,
                        workspaceId: workspaceId,
                        userId: userId,
                        teamId: nil,
                        date: dateStr,
                        hours: hrs,
                        note: shiftNote,
                        latitude: userLoc.coordinate.latitude,
                        longitude: userLoc.coordinate.longitude,
                        targetLatitude: locationManager.targetLatitude,
                        targetLongitude: locationManager.targetLongitude,
                        maxRadiusMeters: locationManager.maxRadiusMeters,
                        isSpoofedLocation: locationManager.isLocationSpoofed,
                        polygonCoordsJson: nil
                    )
                    isClockedIn = true
                    activeShiftStartTime = Date()
                    clockStatusMessage = "Clock-in verified! Report ID: \(report.id)"
                } else {
                    isClockedIn = false
                    clockStatusMessage = "Clock-out completed successfully."
                }
            } catch {
                clockStatusMessage = error.localizedDescription
            }
        }
    }
}

extension DateFormatter {
    static let yyyyMMdd: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }()
}
