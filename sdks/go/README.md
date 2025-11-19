# Windjammer Go SDK

**Go bindings for the Windjammer Game Engine**

[![Go Reference](https://pkg.go.dev/badge/github.com/windjammer/sdk-go.svg)](https://pkg.go.dev/github.com/windjammer/sdk-go)
[![Go](https://img.shields.io/badge/Go-1.21%2B-blue.svg)](https://golang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Features

- 🚀 **Modern Go** - Go 1.21+ with generics
- 🎮 **Complete API** - Full access to all Windjammer features
- ⚡ **High Performance** - Native Rust backend via CGO
- 📦 **Easy Installation** - Simple go get
- 🎨 **2D & 3D** - Support for both 2D and 3D games
- 🔊 **Audio** - 3D spatial audio, mixing, and effects
- 🌐 **Networking** - Client-server, replication, and RPCs

## Installation

```bash
go get github.com/windjammer/sdk-go
```

## Quick Start

### Hello World

```go
package main

import (
    "fmt"
    wj "github.com/windjammer/sdk-go/windjammer"
)

func main() {
    app := wj.NewApp()
    
    app.AddSystem(func() {
        fmt.Println("Hello, Windjammer!")
    })
    
    app.Run()
}
```

### 2D Sprite Example

```go
package main

import wj "github.com/windjammer/sdk-go/windjammer"

func main() {
    app := wj.NewApp()
    
    app.AddStartupSystem(func() {
        camera := wj.NewCamera2D(wj.Vec2{X: 0, Y: 0}, 1.0)
        sprite := wj.NewSprite("sprite.png", wj.Vec2{X: 0, Y: 0}, wj.Vec2{X: 100, Y: 100})
    })
    
    app.AddSystemWithTime(func(time *wj.Time) {
        // Update sprites
    })
    
    app.Run()
}
```

## Documentation

- [API Documentation](https://pkg.go.dev/github.com/windjammer/sdk-go)
- [User Guide](https://windjammer.dev/guide)
- [Examples](https://github.com/windjammer/windjammer/tree/main/sdks/go/examples)

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

