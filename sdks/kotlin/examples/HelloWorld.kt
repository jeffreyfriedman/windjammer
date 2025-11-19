import dev.windjammer.sdk.*

/**
 * Hello World Example
 *
 * The simplest possible Windjammer application in Kotlin.
 *
 * Build and run:
 *   ./gradlew run
 */
fun main() {
    println("=== Windjammer Hello World (Kotlin) ===")
    println("SDK Version: 0.1.0\n")

    // Create a new application
    val app = App()

    // Add a simple system
    app.addSystem {
        println("Hello from the game loop!")
    }

    println("Application created successfully!")
    println("Systems registered: 1\n")
    println("Note: Full app.run() would start the game loop")
    println("For this example, we're just demonstrating SDK setup\n")

    // Run the application
    app.run()
}

