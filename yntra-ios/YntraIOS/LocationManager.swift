import Foundation
import CoreLocation
import Combine

class LocationManager: NSObject, ObservableObject, CLLocationManagerDelegate {
    private let locationManager = CLLocationManager()
    
    @Published var userLocation: CLLocation? = nil
    @Published var authorizationStatus: CLAuthorizationStatus = .notDetermined
    @Published var isWithinGeofence: Bool = false
    @Published var isLocationSpoofed: Bool = false
    @Published var distanceFromTargetMeters: Double = 0.0
    @Published var errorMessage: String? = nil
    
    // Target Geofence Site
    var targetLatitude: Double = 59.3293 // Default Stockholm coordinate
    var targetLongitude: Double = 18.0686
    var maxRadiusMeters: Double = 250.0 // 250m geofence radius
    
    override init() {
        super.init()
        locationManager.delegate = self
        locationManager.desiredAccuracy = kCLLocationAccuracyBest
        locationManager.distanceFilter = 5 // Update every 5 meters
        authorizationStatus = locationManager.authorizationStatus
    }
    
    func requestLocationPermission() {
        locationManager.requestWhenInUseAuthorization()
        locationManager.startUpdatingLocation()
    }
    
    func setTargetSite(latitude: Double, longitude: Double, radiusMeters: Double = 250.0) {
        self.targetLatitude = latitude
        self.targetLongitude = longitude
        self.maxRadiusMeters = radiusMeters
        updateGeofenceStatus()
    }
    
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let location = locations.last else { return }
        DispatchQueue.main.async {
            self.userLocation = location
            self.updateGeofenceStatus()
        }
    }
    
    func locationManager(_ manager: CLLocationManager, didChangeAuthorization status: CLAuthorizationStatus) {
        DispatchQueue.main.async {
            self.authorizationStatus = status
            if status == .authorizedWhenInUse || status == .authorizedAlways {
                self.locationManager.startUpdatingLocation()
            }
        }
    }
    
    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        DispatchQueue.main.async {
            self.errorMessage = error.localizedDescription
        }
    }
    
    private func updateGeofenceStatus() {
        guard let userLoc = userLocation else {
            isWithinGeofence = false
            distanceFromTargetMeters = 999999.0
            return
        }
        
        let targetLocation = CLLocation(latitude: targetLatitude, longitude: targetLongitude)
        let distance = userLoc.distance(from: targetLocation)
        
        if #available(iOS 15.0, *) {
            self.isLocationSpoofed = userLoc.sourceInformation?.isSimulatedBySoftware ?? false
        } else {
            self.isLocationSpoofed = false
        }
        
        self.distanceFromTargetMeters = distance
        self.isWithinGeofence = distance <= maxRadiusMeters
    }
}
