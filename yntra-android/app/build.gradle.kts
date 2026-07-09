plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.yntra.app"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.yntra.app"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.8"
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.activity:activity-compose:1.8.2")
    implementation(platform("androidx.compose:compose-bom:2024.02.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
    
    // JNA is required by UniFFI on Android
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}

// Automatically copy compiled Rust JNI .so libraries to the app's jniLibs folder on build
tasks.register<Copy>("copyRustJniLibs") {
    description = "Copies the compiled Rust shared libraries into the JNI libs folder"
    group = "build"
    
    from("../target/aarch64-linux-android/release") {
        include("libyntra_core.so")
        into("jniLibs/arm64-v8a")
    }
    from("../target/armv7-linux-androideabi/release") {
        include("libyntra_core.so")
        into("jniLibs/armeabi-v7a")
    }
    from("../target/i686-linux-android/release") {
        include("libyntra_core.so")
        into("jniLibs/x86")
    }
    from("../target/x86_64-linux-android/release") {
        include("libyntra_core.so")
        into("jniLibs/x86_64")
    }
    
    into("src/main")
}

tasks.configureEach {
    if (name.startsWith("preBuild")) {
        dependsOn("copyRustJniLibs")
    }
}
