package com.yntra.app.utils

import android.annotation.SuppressLint
import android.content.Context
import android.location.Location
import com.google.android.gms.location.LocationServices
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.yntra_core.TimeReport
import uniffi.yntra_core.clockInGeofenced

class LocationManager(private val context: Context) {
    private val fusedLocationClient = LocationServices.getFusedLocationProviderClient(context)

    var targetLatitude: Double = 59.3293 // Default Stockholm coordinate
    var targetLongitude: Double = 18.0686
    var maxRadiusMeters: Double = 250.0

    @SuppressLint("MissingPermission")
    suspend fun getLastKnownLocation(): Location? = withContext(Dispatchers.IO) {
        try {
            fusedLocationClient.lastLocation.result
        } catch (e: Exception) {
            null
        }
    }

    suspend fun clockIn(
        requesterUserId: String,
        workspaceId: String,
        userId: String,
        teamId: String?,
        date: String,
        hours: Double,
        note: String,
        location: Location,
        polygonCoordsJson: String? = nil
    ): TimeReport = withContext(Dispatchers.IO) {
        val isSpoofed = location.isFromMockProvider

        clockInGeofenced(
            requesterUserId = requesterUserId,
            workspaceId = workspaceId,
            userId = userId,
            teamId = teamId,
            date = date,
            hours = hours,
            note = note,
            latitude = location.latitude,
            longitude = location.longitude,
            targetLatitude = targetLatitude,
            targetLongitude = targetLongitude,
            maxRadiusMeters = maxRadiusMeters,
            isSpoofedLocation = isSpoofed,
            polygonCoordsJson = polygonCoordsJson
        )
    }
}
