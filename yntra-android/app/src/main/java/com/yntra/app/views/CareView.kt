package com.yntra.app.views

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.yntra.app.viewmodels.CareViewModel
import uniffi.yntra_core.ClientProfile
import uniffi.yntra_core.JournalEntry
import uniffi.yntra_core.MedicationItem

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CareView(viewModel: CareViewModel) {
    val clients by viewModel.clients.collectAsState()
    val errorMessage by viewModel.errorMessage.collectAsState()

    var selectedClient by remember { mutableStateOf<ClientProfile?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF0B0F19))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "Care Assistance Clients",
            color = Color.White,
            fontSize = 26.sp,
            fontWeight = FontWeight.Black
        )

        errorMessage?.let {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text(
                    text = it,
                    color = MaterialTheme.colorScheme.onErrorContainer,
                    modifier = Modifier.padding(12.dp),
                    fontSize = 14.sp
                )
            }
        }

        if (clients.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f)
                    .background(Color(0xFF1E293B), RoundedCornerShape(16.dp)),
                contentAlignment = Alignment.Center
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Favorite,
                        contentDescription = "Empty",
                        tint = Color(0xFF94A3B8),
                        modifier = Modifier.size(36.dp)
                    )
                    Text(text = "No care assistance clients registered", color = Color(0xFF94A3B8), fontSize = 14.sp)
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                items(clients) { client ->
                    ClientCard(client = client, onClick = {
                        selectedClient = client
                        viewModel.loadHealthRecords(client.id)
                    })
                }
            }
        }
    }

    selectedClient?.let { client ->
        ClientDetailDialog(
            client = client,
            viewModel = viewModel,
            onDismiss = { selectedClient = null }
        )
    }
}

@Composable
fun ClientCard(client: ClientProfile, onClick: () -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(
                verticalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                Text(
                    text = "${client.firstName} ${client.lastName}",
                    color = Color.White,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Personal ID: ${client.personalNumber ?: "Redacted"}",
                    color = Color(0xFF94A3B8),
                    fontSize = 12.sp
                )
            }

            Surface(
                color = Color(0xFF8B5CF6).copy(alpha = 0.15f),
                shape = RoundedCornerShape(8.dp)
            ) {
                Text(
                    text = client.careLevel ?: "STANDARD CARE",
                    color = Color(0xFF8B5CF6),
                    fontSize = 10.sp,
                    fontWeight = FontWeight.Black,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                )
            }
        }
    }
}

@Composable
fun ClientDetailDialog(
    client: ClientProfile,
    viewModel: CareViewModel,
    onDismiss: () -> Unit
) {
    val journals by viewModel.journals.collectAsState()
    val medications by viewModel.medications.collectAsState()

    var activeTab by remember { mutableStateOf(0) } // 0 -> Journals, 1 -> Medications

    var showAddMedDialog by remember { mutableStateOf(false) }
    var newJournalText by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = Color(0xFF1E293B),
        title = {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = "${client.firstName} ${client.lastName}",
                        color = Color.White,
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "Care Level: ${client.careLevel ?: "Standard"}",
                        color = Color(0xFF94A3B8),
                        fontSize = 12.sp
                    )
                }
                IconButton(onClick = onDismiss) {
                    Icon(imageVector = Icons.Default.Close, contentDescription = "Close", tint = Color.White)
                }
            }
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                // Tab Selection
                TabRow(
                    selectedTabIndex = activeTab,
                    containerColor = Color(0xFF1E293B),
                    contentColor = Color(0xFF8B5CF6)
                ) {
                    Tab(
                        selected = activeTab == 0,
                        onClick = { activeTab = 0 },
                        text = { Text("Daily Journal", color = if (activeTab == 0) Color.White else Color(0xFF94A3B8)) }
                    )
                    Tab(
                        selected = activeTab == 1,
                        onClick = { activeTab = 1 },
                        text = { Text("Medications", color = if (activeTab == 1) Color.White else Color(0xFF94A3B8)) }
                    )
                }

                if (activeTab == 0) {
                    // Journals tab
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .weight(1f, fill = false)
                            .heightIn(max = 240.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        if (journals.isEmpty()) {
                            item {
                                Text(
                                    text = "No care journal records found.",
                                    color = Color(0xFF94A3B8),
                                    fontSize = 13.sp,
                                    modifier = Modifier.padding(vertical = 12.dp)
                                )
                            }
                        } else {
                            items(journals) { entry ->
                                Card(
                                    colors = CardDefaults.cardColors(containerColor = Color(0xFF334155).copy(alpha = 0.5f)),
                                    shape = RoundedCornerShape(12.dp)
                                ) {
                                    Column(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .padding(12.dp),
                                        verticalArrangement = Arrangement.spacedBy(4.dp)
                                    ) {
                                        Text(
                                            text = entry.content,
                                            color = Color.White,
                                            fontSize = 13.sp
                                        )
                                        Row(
                                            modifier = Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.SpaceBetween
                                        ) {
                                            Text(
                                                text = "By: ${entry.author_id ?: "Assistant"}",
                                                color = Color(0xFF94A3B8),
                                                fontSize = 10.sp
                                            )
                                            Text(
                                                text = entry.created_at,
                                                color = Color(0xFF94A3B8),
                                                fontSize = 10.sp
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Log new journal log
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedTextField(
                            value = newJournalText,
                            onValueChange = { newJournalText = it },
                            placeholder = { Text("Log daily care progress...") },
                            modifier = Modifier.weight(1f),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = Color(0xFF8B5CF6),
                                unfocusedBorderColor = Color(0xFF334155),
                                focusedTextColor = Color.White,
                                unfocusedTextColor = Color.White,
                                placeholderColor = Color(0xFF94A3B8)
                            )
                        )
                        IconButton(
                            onClick = {
                                if (newJournalText.isNotBlank()) {
                                    viewModel.addJournal(client.id, newJournalText)
                                    newJournalText = ""
                                }
                            },
                            colors = IconButtonDefaults.iconButtonColors(containerColor = Color(0xFF8B5CF6))
                        ) {
                            Icon(imageVector = Icons.Default.Send, contentDescription = "Send", tint = Color.White)
                        }
                    }
                } else {
                    // Medications tab
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .weight(1f, fill = false)
                            .heightIn(max = 240.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        if (medications.isEmpty()) {
                            item {
                                Text(
                                    text = "No medications logged.",
                                    color = Color(0xFF94A3B8),
                                    fontSize = 13.sp,
                                    modifier = Modifier.padding(vertical = 12.dp)
                                )
                            }
                        } else {
                            items(medications) { med ->
                                Card(
                                    colors = CardDefaults.cardColors(containerColor = Color(0xFF334155).copy(alpha = 0.5f)),
                                    shape = RoundedCornerShape(12.dp)
                                ) {
                                    Column(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .padding(12.dp),
                                        verticalArrangement = Arrangement.spacedBy(4.dp)
                                    ) {
                                        Text(
                                            text = med.name,
                                            color = Color.White,
                                            fontSize = 14.sp,
                                            fontWeight = FontWeight.Bold
                                        )
                                        Text(
                                            text = "Dosage: ${med.dosage ?: "N/A"} | Frequency: ${med.frequency ?: "N/A"}",
                                            color = Color(0xFFCBD5E1),
                                            fontSize = 12.sp
                                        )
                                        if (med.instructions != null) {
                                            Text(
                                                text = "Instructions: ${med.instructions}",
                                                color = Color(0xFF94A3B8),
                                                fontSize = 11.sp
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Button(
                        onClick = { showAddMedDialog = true },
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF8B5CF6))
                    ) {
                        Icon(imageVector = Icons.Default.Add, contentDescription = "Add", tint = Color.White)
                        Spacer(modifier = Modifier.width(6.dp))
                        Text("Add Medication", color = Color.White)
                    }
                }
            }
        },
        confirmButton = {}
    )

    if (showAddMedDialog) {
        AddMedicationDialog(
            onDismiss = { showAddMedDialog = false },
            onSubmit = { name, dosage, frequency, instructions ->
                viewModel.addMed(client.id, name, dosage, frequency, instructions)
                showAddMedDialog = false
            }
        )
    }
}

@Composable
fun AddMedicationDialog(
    onDismiss: () -> Unit,
    onSubmit: (String, String, String, String) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var dosage by remember { mutableStateOf("") }
    var frequency by remember { mutableStateOf("") }
    var instructions by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = Color(0xFF1E293B),
        title = {
            Text("Add Medication Plan", color = Color.White, fontWeight = FontWeight.Bold)
        },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Medication Name") },
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155),
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedLabelColor = Color(0xFF8B5CF6),
                        unfocusedLabelColor = Color(0xFF94A3B8)
                    )
                )

                OutlinedTextField(
                    value = dosage,
                    onValueChange = { dosage = it },
                    label = { Text("Dosage (e.g. 500mg)") },
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155),
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedLabelColor = Color(0xFF8B5CF6),
                        unfocusedLabelColor = Color(0xFF94A3B8)
                    )
                )

                OutlinedTextField(
                    value = frequency,
                    onValueChange = { frequency = it },
                    label = { Text("Frequency (e.g. Twice Daily)") },
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155),
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedLabelColor = Color(0xFF8B5CF6),
                        unfocusedLabelColor = Color(0xFF94A3B8)
                    )
                )

                OutlinedTextField(
                    value = instructions,
                    onValueChange = { instructions = it },
                    label = { Text("Instructions") },
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = Color(0xFF8B5CF6),
                        unfocusedBorderColor = Color(0xFF334155),
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        focusedLabelColor = Color(0xFF8B5CF6),
                        unfocusedLabelColor = Color(0xFF94A3B8)
                    )
                )
            }
        },
        confirmButton = {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Button(
                    onClick = onDismiss,
                    colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent)
                ) {
                    Text("Cancel", color = Color(0xFF94A3B8))
                }
                Button(
                    onClick = {
                        if (name.isNotBlank()) {
                            onSubmit(name, dosage, frequency, instructions)
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF8B5CF6))
                ) {
                    Text("Add", color = Color.White)
                }
            }
        }
    )
}
