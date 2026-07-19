plugins {
    id("com.android.application")
}

android {
    namespace = "com.builder106.asciiarcade"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.builder106.asciiarcade"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    // AGP 9's built-in Kotlin support reads the JVM target from here directly
    // (kotlin.compilerOptions.jvmTarget defaults to this) — no separate
    // kotlinOptions {} block needed.
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
}

dependencies {
    // Intentionally empty for the vertical slice — a plain Activity +
    // SurfaceView + WallpaperService needs no AndroidX/Compose. Add
    // dependencies here when the real scene/theme picker UI lands.
}

// Mirrors shells/ios/project.yml's preBuildScripts hook: only rebuild the
// native library if it's missing, not on every build. Rerun
// scripts/build-android.sh by hand after changing Rust sources — this is a
// "don't forget the .so entirely on a fresh clone" guard, not an
// incremental-rebuild trigger.
val repoRoot = rootDir.parentFile.parentFile
val buildAaFfi = tasks.register<Exec>("buildAaFfi") {
    workingDir = repoRoot
    commandLine("bash", "scripts/build-android.sh")
    onlyIf { !file("src/main/jniLibs/arm64-v8a/libaa_ffi.so").exists() }
}
tasks.named("preBuild") { dependsOn(buildAaFfi) }
