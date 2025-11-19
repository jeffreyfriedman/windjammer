# Windjammer Kotlin SDK

**Kotlin bindings for the Windjammer Game Engine**

[![Maven Central](https://img.shields.io/maven-central/v/dev.windjammer/windjammer-sdk.svg)](https://search.maven.org/artifact/dev.windjammer/windjammer-sdk)
[![Kotlin](https://img.shields.io/badge/Kotlin-1.9%2B-blue.svg)](https://kotlinlang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🚀 **Modern Kotlin** - Kotlin 1.9+ with coroutines and DSL support
- 🎮 **Complete API** - Full access to all Windjammer features
- ⚡ **High Performance** - Native Rust backend via JNI
- 📦 **Easy Installation** - Simple Gradle/Maven dependency
- 🤖 **Android First** - Built for Android game development
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs

## Installation

### Gradle (Kotlin DSL)

```kotlin
dependencies {
    implementation("dev.windjammer:windjammer-sdk:0.1.0")
}
```

### Gradle (Groovy)

```groovy
dependencies {
    implementation 'dev.windjammer:windjammer-sdk:0.1.0'
}
```

### Maven

```xml
<dependency>
    <groupId>dev.windjammer</groupId>
    <artifactId>windjammer-sdk</artifactId>
    <version>0.1.0</version>
</dependency>
```

## Quick Start

### Hello World

```kotlin
import dev.windjammer.sdk.*

fun main() {
    val app = App()
    
    app.addSystem {
        println("Hello, Windjammer!")
    }
    
    app.run()
}
```

### 2D Sprite Example

```kotlin
import dev.windjammer.sdk.*

fun main() {
    val app = App()
    
    app.addStartupSystem {
        val camera = Camera2D(Vec2(0f, 0f), 1.0f)
        val sprite = Sprite("sprite.png", Vec2(0f, 0f), Vec2(100f, 100f))
    }
    
    app.addSystem { time ->
        // Update sprites
    }
    
    app.run()
}
```

### DSL-Style Configuration

```kotlin
import dev.windjammer.sdk.*

fun main() = app {
    startup {
        camera2D(Vec2.ZERO, zoom = 1.0f)
        sprite("sprite.png", position = Vec2.ZERO, size = Vec2(100f, 100f))
    }
    
    system { time ->
        // Update logic
    }
}
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/kotlin)
- [User Guide](https://windjammer.dev/guide)
- [Android Guide](https://windjammer.dev/android)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/kotlin/examples)

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

