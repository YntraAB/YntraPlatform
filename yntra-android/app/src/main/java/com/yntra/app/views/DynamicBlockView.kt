package com.yntra.app.views

import androidx.compose.foundation.background
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
import org.json.JSONArray
import org.json.JSONObject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DynamicBlockView(
    blockId: String,
    blockName: String,
    schemaJson: String,
    dataJson: String,
    onSaveData: (String) -> Unit
) {
    var fieldValues by remember(dataJson) {
        val initialMap = mutableStateMapOf<String, String>()
        try {
            val json = JSONObject(dataJson)
            json.keys().forEach { key ->
                initialMap[key] = json.optString(key, "")
            }
        } catch (e: Exception) {
            // Ignore parse errors
        }
        mutableStateOf(initialMap)
    }

    val schemaFields = remember(schemaJson) {
        val fields = mutableListOf<DynamicFieldSchema>()
        try {
            val arr = JSONArray(schemaJson)
            for (i in 0 until arr.length()) {
                val obj = arr.getJSONObject(i)
                fields.add(
                    DynamicFieldSchema(
                        key = obj.optString("key"),
                        label = obj.optString("label"),
                        type = obj.optString("type", "text"),
                        required = obj.optBoolean("required", false)
                    )
                )
            }
        } catch (e: Exception) {
            // Fallback for non-array schemas
        }
        fields
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF0B0F19))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = blockName,
                color = Color.White,
                fontSize = 26.sp,
                fontWeight = FontWeight.Black
            )

            Button(
                onClick = {
                    val result = JSONObject()
                    fieldValues.value.forEach { (k, v) ->
                        result.put(k, v)
                    }
                    onSaveData(result.toString())
                },
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF4F46E5)),
                shape = RoundedCornerShape(12.dp)
            ) {
                Icon(imageVector = Icons.Default.Check, contentDescription = "Save")
                Spacer(modifier = Modifier.width(6.dp))
                Text("Save Block", fontWeight = FontWeight.Bold)
            }
        }

        if (schemaFields.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f)
                    .background(Color(0xFF1E293B), RoundedCornerShape(16.dp)),
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        imageVector = Icons.Default.Build,
                        contentDescription = "Dynamic",
                        tint = Color(0xFF94A3B8),
                        modifier = Modifier.size(40.dp)
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Dynamic Operational Block ($blockId)",
                        color = Color.White,
                        fontWeight = FontWeight.Bold,
                        fontSize = 16.sp
                    )
                    Text(
                        text = "Schema active and ready for field data inputs.",
                        color = Color(0xFF94A3B8),
                        fontSize = 13.sp
                    )
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                items(schemaFields) { field ->
                    Card(
                        colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
                        shape = RoundedCornerShape(14.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(
                            modifier = Modifier.padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text(
                                text = field.label + if (field.required) " *" else "",
                                color = Color.White,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 14.sp
                            )

                            OutlinedTextField(
                                value = fieldValues.value[field.key] ?: "",
                                onValueChange = { newValue ->
                                    fieldValues.value[field.key] = newValue
                                },
                                modifier = Modifier.fillMaxWidth(),
                                placeholder = { Text("Enter ${field.label}", color = Color(0xFF64748B)) },
                                colors = OutlinedTextFieldDefaults.colors(
                                    focusedTextColor = Color.White,
                                    unfocusedTextColor = Color.White,
                                    focusedBorderColor = Color(0xFF4F46E5),
                                    unfocusedBorderColor = Color(0xFF334155),
                                    focusedContainerColor = Color(0xFF0F172A),
                                    unfocusedContainerColor = Color(0xFF0F172A)
                                ),
                                shape = RoundedCornerShape(10.dp)
                            )
                        }
                    }
                }
            }
        }
    }
}

data class DynamicFieldSchema(
    val key: String,
    val label: String,
    val type: String,
    val required: Boolean
)
