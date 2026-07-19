package com.yntra.app.views

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Send
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.yntra.app.viewmodels.MessagingViewModel
import uniffi.yntra_core.MessageItem

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MessagingView(viewModel: MessagingViewModel) {
    val messages by viewModel.messages.collectAsState()
    val errorMessage by viewModel.errorMessage.collectAsState()

    var activeTab by remember { mutableStateOf("inbox") } // "inbox" | "sent" | "trash"
    var selectedMessage by remember { mutableStateOf<MessageItem?>(null) }
    var showComposeDialog by remember { mutableStateOf(false) }

    var composeSubject by remember { mutableStateOf("") }
    var composeBody by remember { mutableStateOf("") }
    var composeReceiverId by remember { mutableStateOf("") }
    var composeTeamId by remember { mutableStateOf("") }
    var isTeamTarget by remember { mutableStateOf(false) }

    // Filter messages depending on tab
    val filteredMessages = when (activeTab) {
        "inbox" -> messages.filter { it.receiverId == com.yntra.app.SessionManager.activeUserId || it.targetTeamId != null }
        "sent" -> messages.filter { it.senderId == com.yntra.app.SessionManager.activeUserId }
        else -> emptyList() // Trash or fallback
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF0B0F19))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Communications",
                color = Color.White,
                fontSize = 26.sp,
                fontWeight = FontWeight.Black
            )
            FloatingActionButton(
                onClick = { showComposeDialog = true },
                containerColor = Color(0xFF4F46E5),
                contentColor = Color.White,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.size(48.dp)
            ) {
                Icon(imageVector = Icons.Default.Send, contentDescription = "Compose")
            }
        }

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

        // Tab selection row
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            listOf("inbox" to "Inbox", "sent" to "Sent", "trash" to "Trash").forEach { (tabId, label) ->
                val isActive = activeTab == tabId
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .background(
                            if (isActive) Color(0xFF4F46E5) else Color(0xFF1E293B),
                            RoundedCornerShape(12.dp)
                        )
                        .clickable { activeTab = tabId }
                        .padding(vertical = 10.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Text(text = label, color = Color.White, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                }
            }
        }

        // Messages list
        if (filteredMessages.isEmpty()) {
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
                        imageVector = Icons.Default.Email,
                        contentDescription = "Empty",
                        tint = Color(0xFF94A3B8),
                        modifier = Modifier.size(36.dp)
                    )
                    Text(text = "No messages in $activeTab", color = Color(0xFF94A3B8), fontSize = 14.sp)
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                items(filteredMessages) { message ->
                    Card(
                        colors = CardDefaults.cardColors(
                            containerColor = if (message.isRead) Color(0xFF1E293B).copy(alpha = 0.6f) else Color(0xFF1E293B)
                        ),
                        shape = RoundedCornerShape(16.dp),
                        modifier = Modifier.clickable {
                            selectedMessage = message
                            viewModel.markAsRead(message.id)
                        }
                    ) {
                        Column(
                            modifier = Modifier.padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(4.dp)
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    text = message.subject ?: "(No Subject)",
                                    color = Color.White,
                                    fontSize = 15.sp,
                                    fontWeight = if (message.isRead) FontWeight.Medium else FontWeight.Bold
                                )
                                if (!message.isRead) {
                                    Badge(containerColor = Color(0xFF10B981))
                                }
                            }
                            Text(
                                text = message.body?.take(60) ?: "",
                                color = Color(0xFF94A3B8),
                                fontSize = 13.sp
                            )
                        }
                    }
                }
            }
        }

        // Compose Message Modal Dialog
        if (showComposeDialog) {
            AlertDialog(
                onDismissRequest = { showComposeDialog = false },
                title = { Text("Compose Message", color = Color.White, fontWeight = FontWeight.Bold) },
                containerColor = Color(0xFF1E293B),
                text = {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            RadioButton(selected = !isTeamTarget, onClick = { isTeamTarget = false })
                            Text("Direct User", color = Color.White, fontSize = 14.sp)
                            RadioButton(selected = isTeamTarget, onClick = { isTeamTarget = true })
                            Text("Team collective", color = Color.White, fontSize = 14.sp)
                        }

                        if (isTeamTarget) {
                            OutlinedTextField(
                                value = composeTeamId,
                                onValueChange = { composeTeamId = it },
                                label = { Text("Team ID") },
                                modifier = Modifier.fillMaxWidth()
                            )
                        } else {
                            OutlinedTextField(
                                value = composeReceiverId,
                                onValueChange = { composeReceiverId = it },
                                label = { Text("Receiver User ID") },
                                modifier = Modifier.fillMaxWidth()
                            )
                        }

                        OutlinedTextField(
                            value = composeSubject,
                            onValueChange = { composeSubject = it },
                            label = { Text("Subject") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        OutlinedTextField(
                            value = composeBody,
                            onValueChange = { composeBody = it },
                            label = { Text("Body") },
                            modifier = Modifier.fillMaxWidth(),
                            minLines = 3
                        )
                    }
                },
                confirmButton = {
                    Button(
                        onClick = {
                            if (isTeamTarget && composeTeamId.isNotBlank()) {
                                viewModel.sendMessageToTeam(composeTeamId, composeSubject, composeBody)
                            } else if (!isTeamTarget && composeReceiverId.isNotBlank()) {
                                viewModel.sendMessageToUser(composeReceiverId, composeSubject, composeBody)
                            }
                            showComposeDialog = false
                            composeSubject = ""
                            composeBody = ""
                            composeReceiverId = ""
                            composeTeamId = ""
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF4F46E5))
                    ) {
                        Text("Send")
                    }
                },
                dismissButton = {
                    TextButton(onClick = { showComposeDialog = false }) {
                        Text("Cancel", color = Color(0xFF94A3B8))
                    }
                }
            )
        }

        // Message Detail Modal Dialog
        selectedMessage?.let { message ->
            AlertDialog(
                onDismissRequest = { selectedMessage = null },
                title = { Text(message.subject ?: "(No Subject)", color = Color.White, fontWeight = FontWeight.Bold) },
                containerColor = Color(0xFF1E293B),
                text = {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text(
                            text = "From: ${message.senderId ?: "System"}",
                            color = Color(0xFF8B5CF6),
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = message.body ?: "",
                            color = Color.White,
                            fontSize = 14.sp
                        )
                    }
                },
                confirmButton = {
                    Button(onClick = { selectedMessage = null }, colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF4F46E5))) {
                        Text("Close")
                    }
                }
            )
        }
    }
}
