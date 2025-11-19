plugins {
    kotlin("jvm") version "1.9.20"
    `maven-publish`
}

group = "dev.windjammer"
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation(kotlin("stdlib"))
    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    jvmToolchain(17)
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            groupId = "dev.windjammer"
            artifactId = "windjammer-sdk"
            version = "0.1.0"
            
            from(components["java"])
            
            pom {
                name.set("Windjammer SDK")
                description.set("Kotlin SDK for Windjammer Game Engine")
                url.set("https://windjammer.dev")
                
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                    license {
                        name.set("Apache License 2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0")
                    }
                }
                
                developers {
                    developer {
                        name.set("Windjammer Contributors")
                        email.set("contact@windjammer.dev")
                    }
                }
            }
        }
    }
}

