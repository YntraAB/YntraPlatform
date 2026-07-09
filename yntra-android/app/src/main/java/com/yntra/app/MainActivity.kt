package com.yntra.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.*
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.lifecycle.viewmodel.compose.viewModel
import com.yntra.app.viewmodels.*
import com.yntra.app.views.*

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            YntraTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = Color(0xFF0B0F19)
                ) {
                    AppNavigationShell()
                }
            }
        }
    }
}

@Composable
fun YntraTheme(content: @Composable () -> Unit) {
    val darkColorScheme = darkColorScheme(
        primary = Color(0xFF4F46E5),
        secondary = Color(0xFF8B5CF6),
        background = Color(0xFF0B0F19),
        surface = Color(0xFF1E293B)
    )
    MaterialTheme(
        colorScheme = darkColorScheme,
        content = content
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppNavigationShell() {
    val authViewModel: AuthViewModel = viewModel()
    val dashboardViewModel: DashboardViewModel = viewModel()
    val messagingViewModel: MessagingViewModel = viewModel()
    val directoryViewModel: DirectoryViewModel = viewModel()
    val settingsViewModel: SettingsViewModel = viewModel()
    val jobsViewModel: JobsViewModel = viewModel()

    val workspace by settingsViewModel.workspace.collectAsState()
    val isLoggedIn by authViewModel.isLoggedIn.collectAsState()
    var currentScreen by remember { mutableStateOf("home") } // "home" | "messaging" | "directory" | "jobs" | "settings"

    val activeModules = remember(workspace) {
        val modules = mutableMapOf<String, Boolean>()
        workspace?.modules_active?.let { jsonStr ->
            try {
                val jsonObj = org.json.JSONObject(jsonStr)
                val keys = jsonObj.keys()
                while (keys.hasNext()) {
                    val key = keys.next()
                    modules[key] = jsonObj.optBoolean(key, false)
                }
            } catch (e: Exception) {
                // Ignore parsing errors
            }
        }
        modules
    }

    val isMessagingActive = workspace == null || activeModules["messaging"] == true
    val isDirectoryActive = workspace == null || activeModules["directory"] == true
    val isJobsActive = workspace == null || activeModules["jobs"] == true

    LaunchedEffect(isMessagingActive, isDirectoryActive, isJobsActive) {
        if (currentScreen == "messaging" && !isMessagingActive) {
            currentScreen = "home"
        }
        if (currentScreen == "directory" && !isDirectoryActive) {
            currentScreen = "home"
        }
        if (currentScreen == "jobs" && !isJobsActive) {
            currentScreen = "home"
        }
    }

    if (!isLoggedIn) {
        AuthView(viewModel = authViewModel)
    } else {
        Scaffold(
            bottomBar = {
                NavigationBar(
                    containerColor = Color(0xFF1E293B)
                ) {
                    NavigationBarItem(
                        selected = currentScreen == "home",
                        onClick = { currentScreen = "home" },
                        icon = { Icon(imageVector = Icons.Default.Home, contentDescription = "Home") },
                        label = { Text("Home") },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Color.White,
                            selectedTextColor = Color.White,
                            unselectedIconColor = Color(0xFF94A3B8),
                            unselectedTextColor = Color(0xFF94A3B8),
                            indicatorColor = Color(0xFF4F46E5)
                        )
                    )
                    if (isMessagingActive) {
                        NavigationBarItem(
                            selected = currentScreen == "messaging",
                            onClick = { currentScreen = "messaging" },
                            icon = { Icon(imageVector = Icons.Default.Email, contentDescription = "Messages") },
                            label = { Text("Messages") },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                selectedTextColor = Color.White,
                                unselectedIconColor = Color(0xFF94A3B8),
                                unselectedTextColor = Color(0xFF94A3B8),
                                indicatorColor = Color(0xFF4F46E5)
                            )
                        )
                    }
                    if (isDirectoryActive) {
                        NavigationBarItem(
                            selected = currentScreen == "directory",
                            onClick = { currentScreen = "directory" },
                            icon = { Icon(imageVector = Icons.Default.Person, contentDescription = "Directory") },
                            label = { Text("Directory") },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                selectedTextColor = Color.White,
                                unselectedIconColor = Color(0xFF94A3B8),
                                unselectedTextColor = Color(0xFF94A3B8),
                                indicatorColor = Color(0xFF4F46E5)
                            )
                        )
                    }
                    if (isJobsActive) {
                        NavigationBarItem(
                            selected = currentScreen == "jobs",
                            onClick = { currentScreen = "jobs" },
                            icon = { Icon(imageVector = Icons.Default.Build, contentDescription = "Jobs") },
                            label = { Text("Jobs") },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Color.White,
                                selectedTextColor = Color.White,
                                unselectedIconColor = Color(0xFF94A3B8),
                                unselectedTextColor = Color(0xFF94A3B8),
                                indicatorColor = Color(0xFF4F46E5)
                            )
                        )
                    }
                    NavigationBarItem(
                        selected = currentScreen == "settings",
                        onClick = { currentScreen = "settings" },
                        icon = { Icon(imageVector = Icons.Default.Settings, contentDescription = "Settings") },
                        label = { Text("Settings") },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Color.White,
                            selectedTextColor = Color.White,
                            unselectedIconColor = Color(0xFF94A3B8),
                            unselectedTextColor = Color(0xFF94A3B8),
                            indicatorColor = Color(0xFF4F46E5)
                        )
                    )
                }
            }
        ) { paddingValues ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
            ) {
                when (currentScreen) {
                    "home" -> DashboardView(viewModel = dashboardViewModel)
                    "messaging" -> if (isMessagingActive) MessagingView(viewModel = messagingViewModel) else DashboardView(viewModel = dashboardViewModel)
                    "directory" -> if (isDirectoryActive) DirectoryView(viewModel = directoryViewModel) else DashboardView(viewModel = dashboardViewModel)
                    "jobs" -> if (isJobsActive) JobsView(viewModel = jobsViewModel) else DashboardView(viewModel = dashboardViewModel)
                    "settings" -> SettingsView(viewModel = settingsViewModel, onLogout = { authViewModel.logout() })
                }
            }
        }
    }
}
