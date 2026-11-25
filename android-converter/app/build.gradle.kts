plugins {
    id("com.android.application")
}

android {
    namespace = "io.github.brushconv"
    compileSdk = 37

    defaultConfig {
        applicationId = "io.github.brushconv"
        minSdk = 26
        targetSdk = 36
        versionCode = 4
        versionName = "0.2.2"
        // The converter library is built for the phones the app is meant for.
        ndk { abiFilters.add("arm64-v8a") }
    }

    buildTypes {
        release {
            // No shrinking: the app is one screen, and the converter is called through JNA, whose
            // reflection ProGuard would have to be told about.
            isMinifyEnabled = false
            // Without a release keystore the debug key signs it: it installs everywhere, but can't
            // update a published release.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // The generated bindings of libbrushconv_ffi.so call it (UniFFI's Kotlin bindings use JNA).
    implementation("net.java.dev.jna:jna:5.19.1@aar")
    implementation("org.jetbrains.kotlin:kotlin-stdlib:2.3.21")
}
