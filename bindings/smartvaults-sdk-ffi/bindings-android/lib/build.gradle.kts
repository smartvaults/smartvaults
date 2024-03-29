// library version is defined in gradle.properties
val libraryVersion: String by project

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android") version "1.6.10"
    id("maven-publish")
    id("signing")
}

repositories {
    mavenCentral()
    google()
}

android {
    compileSdk = 31

    defaultConfig {
        minSdk = 21
        targetSdk = 31
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
            proguardFiles(file("proguard-android-optimize.txt"), file("proguard-rules.pro"))
        }
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
            withJavadocJar()
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.8.0@aar")
    implementation("org.jetbrains.kotlin:kotlin-stdlib-jdk7")
    implementation("androidx.appcompat:appcompat:1.4.0")
    implementation("androidx.core:core-ktx:1.7.0")
}

afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("maven") {
                groupId = "io.smartvaults"
                artifactId = "smartvaults-sdk"
                version = libraryVersion

                from(components["release"])
                pom {
                    name.set("smartvaults-sdk")
                    description.set("Smart Vaults SDK Kotlin language bindings.")
                    url.set("https://github.com/smartvaults/smartvaults")
                    licenses {
                        license {
                            name.set("MIT")
                            url.set("https://github.com/smartvaults/smartvaults/blob/master/LICENSE")
                        }
                    }
                    developers {
                        developer {
                            id.set("yukibtc")
                            name.set("Yuki Kishimoto")
                            email.set("yukikishimoto@protonmail.com")
                        }
                    }
                    scm {
                        connection.set("scm:git:github.com/smartvaults/smartvaults.git")
                        developerConnection.set("scm:git:ssh://github.com/smartvaults/smartvaults.git")
                        url.set("https://github.com/smartvaults/smartvaults/tree/master")
                    }
                }
            }
        }
    }
}

signing {
    useGpgCmd()
    sign(publishing.publications)
}
