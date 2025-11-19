# Windjammer Java SDK

**Java bindings for the Windjammer Game Engine**

[![Maven Central](https://img.shields.io/maven-central/v/dev.windjammer/windjammer-sdk.svg)](https://search.maven.org/artifact/dev.windjammer/windjammer-sdk)
[![Java](https://img.shields.io/badge/Java-17%2B-blue.svg)](https://www.java.com/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- ☕ **Modern Java** - Java 17+ with records and sealed classes
- 🎮 **Complete API** - Full access to all Windjammer features
- 🚀 **High Performance** - Native Rust backend via JNI
- 📦 **Easy Installation** - Simple Maven/Gradle dependency
- 🤖 **Android Support** - Build games for Android
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs

## Installation

### Maven

```xml
<dependency>
    <groupId>dev.windjammer</groupId>
    <artifactId>windjammer-sdk</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle

```gradle
implementation 'dev.windjammer:windjammer-sdk:0.1.0'
```

## Quick Start

### Hello World

```java
import dev.windjammer.sdk.*;

public class HelloWorld {
    public static void main(String[] args) {
        var app = new App();
        
        app.addSystem(() -> {
            System.out.println("Hello, Windjammer!");
        });
        
        app.run();
    }
}
```

### 2D Sprite Example

```java
import dev.windjammer.sdk.*;

public class SpriteDemo {
    public static void main(String[] args) {
        var app = new App();
        
        app.addStartupSystem(() -> {
            var camera = new Camera2D(new Vec2(0, 0), 1.0f);
            var sprite = new Sprite("sprite.png", new Vec2(0, 0), new Vec2(100, 100));
        });
        
        app.addSystem((Time time) -> {
            // Update sprites
        });
        
        app.run();
    }
}
```

## Documentation

- [API Documentation](https://windjammer.dev/docs/java)
- [User Guide](https://windjammer.dev/guide)
- [Android Guide](https://windjammer.dev/android)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/java/examples)

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

