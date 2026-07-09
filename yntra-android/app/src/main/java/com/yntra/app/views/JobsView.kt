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
import com.yntra.app.viewmodels.JobsViewModel
import uniffi.yntra_core.JobTicket
import org.json.JSONArray
import org.json.JSONObject

data class ChecklistTask(val text: String, val done: Boolean)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun JobsView(viewModel: JobsViewModel) {
    val jobs by viewModel.jobs.collectAsState()
    val errorMessage by viewModel.errorMessage.collectAsState()

    var selectedJob by remember { mutableStateOf<JobTicket?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF0B0F19))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "Logistics Work Orders",
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

        if (jobs.isEmpty()) {
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
                        imageVector = Icons.Default.Build,
                        contentDescription = "Empty",
                        tint = Color(0xFF94A3B8),
                        modifier = Modifier.size(36.dp)
                    )
                    Text(text = "No job tickets assigned", color = Color(0xFF94A3B8), fontSize = 14.sp)
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                items(jobs) { job ->
                    JobTicketCard(job = job, onClick = { selectedJob = job })
                }
            }
        }
    }

    selectedJob?.let { job ->
        JobDetailDialog(
            job = job,
            viewModel = viewModel,
            onDismiss = { selectedJob = null }
        )
    }
}

@Composable
fun JobTicketCard(job: JobTicket, onClick: () -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = job.title,
                    color = Color.White,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f)
                )
                
                Spacer(modifier = Modifier.width(8.dp))
                StatusBadge(status = job.status)
            }

            Text(
                text = job.description,
                color = Color(0xFF94A3B8),
                fontSize = 13.sp,
                maxLines = 2
            )

            Divider(color = Color(0xFF334155), thickness = 1.dp)

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.LocationOn,
                        contentDescription = "Address",
                        tint = Color(0xFF8B5CF6),
                        modifier = Modifier.size(16.dp)
                    )
                    Text(
                        text = job.locationAddress,
                        color = Color(0xFFCBD5E1),
                        fontSize = 12.sp
                    )
                }

                Text(
                    text = job.scheduledDate,
                    color = Color(0xFF8B5CF6),
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold
                )
            }
        }
    }
}

@Composable
fun StatusBadge(status: String) {
    val (bgColor, textColor) = when (status) {
        "assigned" -> Color(0xFFFEF3C7) to Color(0xFFD97706)
        "in_progress" -> Color(0xFFDBEAFE) to Color(0xFF2563EB)
        "completed" -> Color(0xFFD1FAE5) to Color(0xFF059669)
        else -> Color(0xFFF3F4F6) to Color(0xFF4B5563)
    }

    Surface(
        color = bgColor,
        shape = RoundedCornerShape(8.dp)
    ) {
        Text(
            text = status.uppercase().replace("_", " "),
            color = textColor,
            fontSize = 10.sp,
            fontWeight = FontWeight.Black,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
        )
    }
}

@Composable
fun JobDetailDialog(
    job: JobTicket,
    viewModel: JobsViewModel,
    onDismiss: () -> Unit
) {
    var reportText by remember { mutableStateOf("") }
    val tasks = remember(job.checklistJson) {
        val list = mutableStateListOf<ChecklistTask>()
        try {
            val jsonArr = JSONArray(job.checklistJson)
            for (i in 0 until jsonArr.length()) {
                val obj = jsonArr.getJSONObject(i)
                list.add(ChecklistTask(obj.getString("text"), obj.optBoolean("done", false)))
            }
        } catch (e: Exception) {
            // Fallback
        }
        list
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = Color(0xFF1E293B),
        title = {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = job.title,
                    color = Color.White,
                    fontSize = 18.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f)
                )
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
                // Description
                Text(
                    text = job.description,
                    color = Color(0xFFCBD5E1),
                    fontSize = 14.sp
                )

                Divider(color = Color(0xFF334155))

                // Addresses
                if (job.originAddress != null || job.destinationAddress != null) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(text = "ROUTE DETAILS", color = Color(0xFF94A3B8), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        Text(
                            text = "Origin: ${job.originAddress ?: "N/A"} (Floor: ${job.originFloor}, Elevator: ${if (job.originHasElevator) "Yes" else "No"})",
                            color = Color(0xFFCBD5E1),
                            fontSize = 12.sp
                        )
                        Text(
                            text = "Destination: ${job.destinationAddress ?: "N/A"} (Floor: ${job.destinationFloor}, Elevator: ${if (job.destinationHasElevator) "Yes" else "No"})",
                            color = Color(0xFFCBD5E1),
                            fontSize = 12.sp
                        )
                    }
                    Divider(color = Color(0xFF334155))
                }

                // Checklist
                Text(text = "CHECKLIST / TASKS", color = Color(0xFF94A3B8), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                if (tasks.isEmpty()) {
                    Text(text = "No checklist items.", color = Color(0xFF94A3B8), fontSize = 13.sp)
                } else {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        tasks.forEachIndexed { index, task ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable(enabled = job.status == "in_progress") {
                                        tasks[index] = task.copy(done = !task.done)
                                    },
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Checkbox(
                                    checked = task.done,
                                    enabled = job.status == "in_progress",
                                    onCheckedChange = { isChecked ->
                                        tasks[index] = task.copy(done = isChecked)
                                    },
                                    colors = CheckboxDefaults.colors(
                                        checkedColor = Color(0xFF8B5CF6),
                                        checkmarkColor = Color.White
                                    )
                                )
                                Text(
                                    text = task.text,
                                    color = if (task.done) Color(0xFF94A3B8) else Color.White,
                                    fontSize = 13.sp
                                )
                            }
                        }
                    }
                }

                // Completion Report Section
                if (job.status == "in_progress") {
                    Divider(color = Color(0xFF334155))
                    Text(text = "SUBMIT COMPLETION REPORT", color = Color(0xFF94A3B8), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    OutlinedTextField(
                        value = reportText,
                        onValueChange = { reportText = it },
                        placeholder = { Text("Describe the completed work...") },
                        modifier = Modifier.fillMaxWidth(),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = Color(0xFF8B5CF6),
                            unfocusedBorderColor = Color(0xFF334155),
                            focusedTextColor = Color.White,
                            unfocusedTextColor = Color.White,
                            placeholderColor = Color(0xFF94A3B8)
                        )
                    )
                } else if (job.status == "completed") {
                    Divider(color = Color(0xFF334155))
                    Text(text = "COMPLETION REPORT", color = Color(0xFF94A3B8), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    Text(
                        text = job.completionReport ?: "No report text submitted.",
                        color = Color(0xFFCBD5E1),
                        fontSize = 13.sp
                    )
                }
            }
        },
        confirmButton = {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                if (job.status == "assigned") {
                    Button(
                        onClick = {
                            viewModel.updateStatus(job.id, "in_progress")
                            onDismiss()
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF8B5CF6))
                    ) {
                        Text("Start Job", color = Color.White)
                    }
                } else if (job.status == "in_progress") {
                    Button(
                        onClick = {
                            // Serialize checklist back to JSON
                            val jsonArr = JSONArray()
                            tasks.forEach { t ->
                                val obj = JSONObject()
                                obj.put("text", t.text)
                                obj.put("done", t.done)
                                jsonArr.put(obj)
                            }
                            viewModel.completeJob(job.id, jsonArr.toString(), reportText)
                            onDismiss()
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981))
                    ) {
                        Text("Complete & Submit", color = Color.White)
                    }
                }
            }
        }
    )
}
