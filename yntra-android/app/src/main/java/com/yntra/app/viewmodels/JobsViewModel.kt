package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class JobsViewModel : ViewModel() {
    private val _jobs = MutableStateFlow<List<JobTicket>>(emptyList())
    val jobs: StateFlow<List<JobTicket>> = _jobs

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        loadJobs()
    }

    init {
        loadJobs()
        registerObserver(observer)
    }

    fun loadJobs() {
        viewModelScope.launch {
            try {
                _jobs.value = getJobTickets(requesterUserId = "user-1")
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun updateStatus(jobId: String, status: String) {
        viewModelScope.launch {
            try {
                updateJobTicketStatus(requesterUserId = "user-1", ticketId = jobId, status = status)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun completeJob(jobId: String, checklistJson: String, completionReport: String) {
        viewModelScope.launch {
            try {
                submitCompletionReport(
                    requesterUserId = "user-1",
                    jobId = jobId,
                    checklistJson = checklistJson,
                    completionReport = completionReport
                )
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
