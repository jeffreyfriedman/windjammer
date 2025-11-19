package dev.windjammer.sdk;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;

/**
 * Main application class for Windjammer games.
 */
public class App {
    private final List<Runnable> systems = new ArrayList<>();
    private final List<Consumer<Time>> systemsWithTime = new ArrayList<>();
    private final List<Runnable> startupSystems = new ArrayList<>();
    private final List<Runnable> shutdownSystems = new ArrayList<>();
    private boolean running = false;

    /**
     * Creates a new Windjammer application.
     */
    public App() {
        System.out.println("[Windjammer] Initializing application...");
    }

    /**
     * Adds a system that runs every frame.
     *
     * @param system The system to add
     * @return This app instance for chaining
     */
    public App addSystem(Runnable system) {
        systems.add(system);
        return this;
    }

    /**
     * Adds a system with time parameter that runs every frame.
     *
     * @param system The system to add
     * @return This app instance for chaining
     */
    public App addSystem(Consumer<Time> system) {
        systemsWithTime.add(system);
        return this;
    }

    /**
     * Adds a startup system that runs once at the beginning.
     *
     * @param system The startup system to add
     * @return This app instance for chaining
     */
    public App addStartupSystem(Runnable system) {
        startupSystems.add(system);
        return this;
    }

    /**
     * Adds a shutdown system that runs once at the end.
     *
     * @param system The shutdown system to add
     * @return This app instance for chaining
     */
    public App addShutdownSystem(Runnable system) {
        shutdownSystems.add(system);
        return this;
    }

    /**
     * Runs the application.
     */
    public void run() {
        System.out.println("[Windjammer] Starting application with " + 
                         (systems.size() + systemsWithTime.size()) + " systems");

        // Run startup systems
        for (var system : startupSystems) {
            try {
                system.run();
            } catch (Exception e) {
                System.err.println("[Windjammer] Error in startup system: " + e.getMessage());
            }
        }

        running = true;

        // TODO: Start actual game loop with JNI
        System.out.println("[Windjammer] Running systems...");
        var time = new Time();
        
        for (var system : systems) {
            try {
                system.run();
            } catch (Exception e) {
                System.err.println("[Windjammer] Error in system: " + e.getMessage());
            }
        }
        
        for (var system : systemsWithTime) {
            try {
                system.accept(time);
            } catch (Exception e) {
                System.err.println("[Windjammer] Error in system: " + e.getMessage());
            }
        }

        // Run shutdown systems
        for (var system : shutdownSystems) {
            try {
                system.run();
            } catch (Exception e) {
                System.err.println("[Windjammer] Error in shutdown system: " + e.getMessage());
            }
        }

        System.out.println("[Windjammer] Application finished");
        running = false;
    }

    /**
     * Checks if the application is currently running.
     *
     * @return true if running, false otherwise
     */
    public boolean isRunning() {
        return running;
    }

    /**
     * Requests the application to quit.
     */
    public void quit() {
        running = false;
        System.out.println("[Windjammer] Quit requested");
    }
}

