package dev.windjammer.sdk

/**
 * Main application class for Windjammer games.
 */
class App {
    private val systems = mutableListOf<() -> Unit>()
    private val systemsWithTime = mutableListOf<(Time) -> Unit>()
    private val startupSystems = mutableListOf<() -> Unit>()
    private val shutdownSystems = mutableListOf<() -> Unit>()
    private var running = false

    init {
        println("[Windjammer] Initializing application...")
    }

    /**
     * Adds a system that runs every frame.
     */
    fun addSystem(system: () -> Unit): App {
        systems.add(system)
        return this
    }

    /**
     * Adds a system with time parameter that runs every frame.
     */
    fun addSystem(system: (Time) -> Unit): App {
        systemsWithTime.add(system)
        return this
    }

    /**
     * Adds a startup system that runs once at the beginning.
     */
    fun addStartupSystem(system: () -> Unit): App {
        startupSystems.add(system)
        return this
    }

    /**
     * Adds a shutdown system that runs once at the end.
     */
    fun addShutdownSystem(system: () -> Unit): App {
        shutdownSystems.add(system)
        return this
    }

    /**
     * Runs the application.
     */
    fun run() {
        println("[Windjammer] Starting application with ${systems.size + systemsWithTime.size} systems")

        // Run startup systems
        startupSystems.forEach { system ->
            try {
                system()
            } catch (e: Exception) {
                System.err.println("[Windjammer] Error in startup system: ${e.message}")
            }
        }

        running = true

        // TODO: Start actual game loop with JNI
        println("[Windjammer] Running systems...")
        val time = Time()

        systems.forEach { system ->
            try {
                system()
            } catch (e: Exception) {
                System.err.println("[Windjammer] Error in system: ${e.message}")
            }
        }

        systemsWithTime.forEach { system ->
            try {
                system(time)
            } catch (e: Exception) {
                System.err.println("[Windjammer] Error in system: ${e.message}")
            }
        }

        // Run shutdown systems
        shutdownSystems.forEach { system ->
            try {
                system()
            } catch (e: Exception) {
                System.err.println("[Windjammer] Error in shutdown system: ${e.message}")
            }
        }

        println("[Windjammer] Application finished")
        running = false
    }

    /**
     * Checks if the application is currently running.
     */
    fun isRunning(): Boolean = running

    /**
     * Requests the application to quit.
     */
    fun quit() {
        running = false
        println("[Windjammer] Quit requested")
    }
}

// DSL-style builders
fun app(block: App.() -> Unit) {
    App().apply(block).run()
}

fun App.startup(block: () -> Unit) = addStartupSystem(block)
fun App.system(block: () -> Unit) = addSystem(block)
fun App.system(block: (Time) -> Unit) = addSystem(block)
fun App.shutdown(block: () -> Unit) = addShutdownSystem(block)

