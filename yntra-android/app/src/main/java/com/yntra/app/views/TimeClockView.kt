package com.yntra.app.views

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.LocationOff
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.yntra.app.utils.AndroidLocationManager
import kotlinx.coroutines.launch
import uniffi.yntra_core.clockInGeofenced

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TimeClockView() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val locationManager = remember { AndroidLocationManager(context) }
    val locationState by locationManager.locationState.collectAsState()

    var hoursText by remember { mutableStateOf("8.0") }
    var noteText by remember { mutableStateOf("Field Shift Work") }
    var isClockedIn by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    DisposableEffect(Unit) {
        locationManager.startLocationUpdates()
        onDispose {
            locationManager.stopLocationUpdates()
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF0B0F19))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "GPS Time Clock",
            color = Color.White,
            fontSize = 26.sp,
            fontWeight = FontWeight.Black
        )
        Text(
            text = "Geofenced Field Shift Clocking",
            color = Color(0xFF94A3B8),
            fontSize = 13.sp
        )

        // Geofence Badge & Location Card
        Card(
            colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
            shape = RoundedCornerShape(16.dp),
            modifier = Modifier
                .fillMaxWidth()
                .border(
                    width = 1.dp,
                    color = if (locationState.isWithinGeofence) Color(0xFF10B981) else Color(0xFFF59E0B),
                    shape = RoundedCornerShape(16.dp)
                )
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Icon(
                        imageVector = if (locationState.isWithinGeofence) Icons.Default.CheckCircle else Icons.Default.LocationOff,
                        contentDescription = "Geofence Status",
                        tint = if (locationState.isWithinGeofence) Color(0xFF10B981) else Color(0xFFF59E0B)
                    )
                    Column {
                        Text(
                            text = if (locationState.isWithinGeofence) "Inside Job Site Geofence" else "Outside Geofence Radius",
                            color = Color.White,
                            fontWeight = FontWeight.Bold,
                            fontSize = 15.sp
                        )
                        Text(
                            text = "Distance: %.1fm from target site".format(locationState.distanceFromTargetMeters),
                            color = Color(0xFF94A3B8),
                            fontSize = 12.sp
                        )
                    }
                }

                if (locationState.latitude != 0.0) {
                    Text(
                        text = "GPS: %.5f, %.5f".format(locationState.latitude, locationState.longitude),
                        color = Color(0xFF64748B),
                        fontSize = 11.sp
                    )
                }
            }
        }

        statusMessage?.let { msg ->
            Text(
                text = msg,
                color = if (msg.contains("Violation") || msg.contains("Failed")) Color.Red else Color.Green,
                fontSize = 13.sp,
                fontWeight = FontWeight.Medium
            )
        }

        // Shift Details Card
        Card(
            colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp)
            ) {
                Text(
                    text = "SHIFT DETAILS",
                    color = Color(0xFF64748B),
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold
                )

                OutlinedTextField(
                    value = noteText,
                    onValueChange = { noteText = it },
                    label = { Text("Shift Note") },
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155)
                    ),
                    modifier = Modifier.fillMaxWidth()
                )

                OutlinedTextField(
                    value = hoursText,
                    onValueChange = { hoursText = it },
                    label = { Text("Logged Hours") },
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155)
                    ),
                    modifier = Modifier.fillMaxWidth()
                )

                Button(
                    onClick = {
                        val hrs = hoursText.toDoubleOrNull() ?: 8.0
                        scope.launch {
                            try {
                                if (!isClockedIn) {
                                    val report = clockInGeofenced(
                                        requesterUserId = "usr_field_01",
                                        workspaceId = "ws_default",
                                        userId = "usr_field_01",
                                        teamId = null,
                                        date = "2026-08-04",
                                        hours = hrs,
                                        note = noteText,
                                        latitude = locationState.latitude,
                                        longitude = locationState.longitude,
                                        targetLatitude = 59.3293,
                                        targetLongitude = 18.0686,
                                        maxRadiusMeters = 250.0,
                                        isSpoofedLocation = locationState.isLocationSpoofed,
                                        polygonCoordsJson = null
                                    )
                                    isClockedIn = true
                                    statusMessage = "Clock-in verified! Report ID: ${report.id}"
                                } else {
                                    isClockedIn = false
                                    statusMessage = "Clock-out completed successfully."
                                }
                            } catch (e: Exception) {
                                statusMessage = e.localizedMessage
                            }
                        }
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (isClockedIn) Color.Red else (if (locationState.isWithinGeofence) Color(0xFF8B5CF6) else Color.Gray)
                    ),
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier.fillMaxWidth(),
                    enabled = locationState.isWithinGeofence || isClockedIn
                ) {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.padding(vertical = 4.dp)
                    ) {
                        Icon(
                            imageVector = if (isClockedIn) Icons.Default.Stop else Icons.Default.PlayArrow,
                            contentDescription = null
                        )
                        Text(
                            text = if (isClockedIn) "Clock Out & Submit Shift" else "GPS Clock In",
                            fontSize = 16.sp,
                            fontWeight = FontWeight.Bold
                        )
                    }
                }
            }
        }
    }
}
