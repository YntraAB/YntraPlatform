package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class CareViewModel : ViewModel() {
    private val _clients = MutableStateFlow<List<ClientProfile>>(emptyList())
    val clients: StateFlow<List<ClientProfile>> = _clients

    private val _journals = MutableStateFlow<List<JournalEntry>>(emptyList())
    val journals: StateFlow<List<JournalEntry>> = _journals

    private val _medications = MutableStateFlow<List<MedicationItem>>(emptyList())
    val medications: StateFlow<List<MedicationItem>> = _medications

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        loadClients()
    }

    init {
        loadClients()
        registerObserver(observer)
    }

    fun loadClients() {
        viewModelScope.launch {
            try {
                _clients.value = getClients(requesterUserId = "user-1")
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun loadHealthRecords(clientId: String) {
        viewModelScope.launch {
            try {
                _journals.value = getJournals(clientId = clientId, actorId = "user-1")
                _medications.value = getMedications(clientId = clientId, actorId = "user-1")
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun addJournal(clientId: String, content: String) {
        viewModelScope.launch {
            try {
                addJournalEntry(actorId = "user-1", clientId = clientId, content = content)
                loadHealthRecords(clientId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun addMed(clientId: String, name: String, dosage: String, frequency: String, instructions: String) {
        viewModelScope.launch {
            try {
                addMedication(actorId = "user-1", clientId = clientId, name = name, dosage = dosage, frequency = frequency, instructions = instructions)
                loadHealthRecords(clientId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    override fun onCleared() {
        super.onCleared()
        clearObservers()
    }
}
