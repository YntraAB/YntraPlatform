package com.yntra.app.utils

import android.content.Context
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlin.math.*

data class LocationState(
    val latitude: Double = 0.0,
    val longitude: Double = 0.0,
    val isWithinGeofence: Boolean = false,
    val isLocationSpoofed: Boolean = false,
    val distanceFromTargetMeters: Double = 0.0,
    val hasLocationPermission: Boolean = false,
    val errorMessage: String? = null
)

class AndroidLocationManager(private val context: Context) : LocationListener {
    private val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager

    private val _locationState = MutableStateFlow(LocationState())
    val locationState: StateFlow<LocationState> = _locationState

    private var targetLatitude: Double = 59.3293
    private var targetLongitude: Double = 18.0686
    private var maxRadiusMeters: Double = 250.0

    fun setTargetSite(lat: Double, lon: Double, radiusMeters: Double = 250.0) {
        targetLatitude = lat
        targetLongitude = lon
        maxRadiusMeters = radiusMeters
        updateGeofence()
    }

    fun startLocationUpdates() {
        try {
            val isGpsEnabled = locationManager.isProviderEnabled(LocationManager.GPS_PROVIDER)
            val isNetEnabled = locationManager.isProviderEnabled(LocationManager.NETWORK_PROVIDER)

            if (isGpsEnabled) {
                locationManager.requestLocationUpdates(LocationManager.GPS_PROVIDER, 5000L, 5f, this)
            } else if (isNetEnabled) {
                locationManager.requestLocationUpdates(LocationManager.NETWORK_PROVIDER, 5000L, 5f, this)
            }
            
            val lastKnown = locationManager.getLastKnownLocation(LocationManager.GPS_PROVIDER)
                ?: locationManager.getLastKnownLocation(LocationManager.NETWORK_PROVIDER)
            lastKnown?.let { onLocationChanged(it) }
        } catch (e: SecurityException) {
            _locationState.value = _locationState.value.copy(
                errorMessage = "Location permission required: ${e.localizedMessage}"
            )
        }
    }

    fun stopLocationUpdates() {
        try {
            locationManager.removeUpdates(this)
        } catch (e: SecurityException) {
            // Ignore
        }
    }

    override fun onLocationChanged(location: Location) {
        val distance = calculateHaversineDistance(
            location.latitude, location.longitude,
            targetLatitude, targetLongitude
        )
        val within = distance <= maxRadiusMeters
        val isMock = location.isFromMockProvider

        _locationState.value = _locationState.value.copy(
            latitude = location.latitude,
            longitude = location.longitude,
            distanceFromTargetMeters = distance,
            isWithinGeofence = within,
            isLocationSpoofed = isMock,
            errorMessage = if (isMock) "Security Warning: Spoofed GPS location active" else null
        )
    }

    private fun updateGeofence() {
        val current = _locationState.value
        val distance = calculateHaversineDistance(
            current.latitude, current.longitude,
            targetLatitude, targetLongitude
        )
        _locationState.value = current.copy(
            distanceFromTargetMeters = distance,
            isWithinGeofence = distance <= maxRadiusMeters
        )
    }

    private fun calculateHaversineDistance(lat1: Double, lon1: Double, lat2: Double, lon2: Double): Double {
        val r = 6371000.0 // Earth radius in meters
        val dLat = Math.toRadians(lat2 - lat1)
        val dLon = Math.toRadians(lon2 - lon1)
        val a = sin(dLat / 2.0).pow(2.0) +
                cos(Math.toRadians(lat1)) * cos(Math.toRadians(lat2)) *
                sin(dLon / 2.0).pow(2.0)
        val c = 2.0 * atan2(sqrt(a), sqrt(1.0 - a))
        return r * c
    }

    override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) {}
    override fun onProviderEnabled(provider: String) {}
    override fun onProviderDisabled(provider: String) {}
}
