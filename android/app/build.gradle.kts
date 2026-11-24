plugins {
    id("com.android.application")
    kotlin("android")
}

// The ABIs the APK carries: phones and tablets, and the x86_64 emulator.
val abis = listOf("arm64-v8a", "x86_64")
// Android 12, the oldest release the GPUI backend (gpui_android) supports.
val minSdkVersion = 31

// Builds libfastemu_android.so for every ABI with cargo-ndk into jniLibs,
// which Gradle then packages. Cargo decides what is up to date. Skip it with
// -Pfastemu.skipCargo when the libraries are already in place.
val workspaceDir = rootDir.parentFile
val jniLibsDir = file("src/main/jniLibs")
val cargoBuild by tasks.registering(Exec::class) {
    description = "Builds the native library with cargo-ndk for every ABI."
    group = "build"
    workingDir = workspaceDir
    val targets = abis.flatMap { listOf("-t", it) }
    commandLine(
        listOf("cargo", "ndk") + targets + listOf(
            "-P", minSdkVersion.toString(),
            "-o", jniLibsDir.absolutePath,
            "build", "--release", "-p", "fastemu-android",
        ),
    )
    onlyIf { !project.hasProperty("fastemu.skipCargo") }
}
tasks.named("preBuild") { dependsOn(cargoBuild) }

android {
    namespace = "io.github.fzn0x.fastemu"
    compileSdk = 35
    buildToolsVersion = "35.0.0"
    // Only strips the packaged libraries; cargo-ndk picks its own NDK.
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "io.github.fzn0x.fastemu"
        minSdk = minSdkVersion
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk {
            abiFilters += abis
        }
    }

    buildTypes {
        getByName("release") {
            // R8 drops the parts of AppCompat the app never uses, most of
            // the Java side's size; proguard-rules.pro keeps what Rust
            // calls by name.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            // Debug-signed until FastEmu has a release key; see
            // docs/building-android.md.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    // android-activity 0.6 carries GameActivity's 4.4 native code, which
    // works with the 4.0 Java side; the Zed Android port found that 4.4's
    // Java side attaches input listeners that leak captured mouse events.
    implementation("androidx.games:games-activity:4.4.2")
    implementation("androidx.appcompat:appcompat:1.8.0")
    implementation("androidx.core:core-ktx:1.13.1")
}
