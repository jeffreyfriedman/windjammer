import dev.windjammer.sdk.*

/**
 * Hello World Example (DSL Style)
 *
 * Demonstrates Kotlin's DSL capabilities for a more idiomatic API.
 *
 * Build and run:
 *   ./gradlew run
 */
fun main() {
    println("=== Windjammer Hello World DSL (Kotlin) ===")
    println("SDK Version: 0.1.0\n")

    // Create and run application using DSL
    app {
        startup {
            println("Starting up...")
        }

        system {
            println("Hello from the game loop!")
        }

        system { time ->
            println("Frame ${time.frameCount}: delta=${time.deltaSeconds}s")
        }

        shutdown {
            println("Shutting down...")
        }
    }
}

