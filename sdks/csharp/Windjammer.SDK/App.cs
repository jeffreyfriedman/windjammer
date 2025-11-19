using System;
using System.Collections.Generic;

namespace Windjammer.SDK;

/// <summary>
/// System function delegate - runs every frame.
/// </summary>
public delegate void SystemFunc();

/// <summary>
/// System function delegate with time parameter - runs every frame.
/// </summary>
public delegate void SystemFuncWithTime(Time time);

/// <summary>
/// Startup system function delegate - runs once at startup.
/// </summary>
public delegate void StartupSystemFunc();

/// <summary>
/// Shutdown system function delegate - runs once at shutdown.
/// </summary>
public delegate void ShutdownSystemFunc();

/// <summary>
/// Main application class for Windjammer games.
/// </summary>
/// <example>
/// <code>
/// var app = new App();
/// app.AddSystem(() => Console.WriteLine("Update!"));
/// app.Run();
/// </code>
/// </example>
public class App
{
    private readonly List<SystemFunc> _systems = new();
    private readonly List<SystemFuncWithTime> _systemsWithTime = new();
    private readonly List<StartupSystemFunc> _startupSystems = new();
    private readonly List<ShutdownSystemFunc> _shutdownSystems = new();
    private bool _running;

    /// <summary>
    /// Creates a new Windjammer application.
    /// </summary>
    public App()
    {
        Console.WriteLine("[Windjammer] Initializing application...");
    }

    /// <summary>
    /// Adds a system that runs every frame.
    /// </summary>
    /// <param name="system">The system function to add.</param>
    /// <returns>This app instance for chaining.</returns>
    /// <example>
    /// <code>
    /// app.AddSystem(() =>
    /// {
    ///     Console.WriteLine("Update!");
    /// });
    /// </code>
    /// </example>
    public App AddSystem(SystemFunc system)
    {
        _systems.Add(system);
        return this;
    }

    /// <summary>
    /// Adds a system with time parameter that runs every frame.
    /// </summary>
    /// <param name="system">The system function to add.</param>
    /// <returns>This app instance for chaining.</returns>
    /// <example>
    /// <code>
    /// app.AddSystem((time) =>
    /// {
    ///     Console.WriteLine($"Delta: {time.DeltaSeconds}");
    /// });
    /// </code>
    /// </example>
    public App AddSystem(SystemFuncWithTime system)
    {
        _systemsWithTime.Add(system);
        return this;
    }

    /// <summary>
    /// Adds a startup system that runs once at the beginning.
    /// </summary>
    /// <param name="system">The startup system function to add.</param>
    /// <returns>This app instance for chaining.</returns>
    /// <example>
    /// <code>
    /// app.AddStartupSystem(() =>
    /// {
    ///     Console.WriteLine("Setup!");
    /// });
    /// </code>
    /// </example>
    public App AddStartupSystem(StartupSystemFunc system)
    {
        _startupSystems.Add(system);
        return this;
    }

    /// <summary>
    /// Adds a shutdown system that runs once at the end.
    /// </summary>
    /// <param name="system">The shutdown system function to add.</param>
    /// <returns>This app instance for chaining.</returns>
    /// <example>
    /// <code>
    /// app.AddShutdownSystem(() =>
    /// {
    ///     Console.WriteLine("Cleanup!");
    /// });
    /// </code>
    /// </example>
    public App AddShutdownSystem(ShutdownSystemFunc system)
    {
        _shutdownSystems.Add(system);
        return this;
    }

    /// <summary>
    /// Runs the application.
    /// This starts the game loop and runs until the application is closed.
    /// </summary>
    public void Run()
    {
        Console.WriteLine($"[Windjammer] Starting application with {_systems.Count + _systemsWithTime.Count} systems");

        // Run startup systems
        foreach (var system in _startupSystems)
        {
            try
            {
                system();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Windjammer] Error in startup system: {ex}");
            }
        }

        _running = true;

        // TODO: Start actual game loop with FFI
        // For now, just run systems once as a demonstration
        Console.WriteLine("[Windjammer] Running systems...");
        var time = new Time();
        
        foreach (var system in _systems)
        {
            try
            {
                system();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Windjammer] Error in system: {ex}");
            }
        }
        
        foreach (var system in _systemsWithTime)
        {
            try
            {
                system(time);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Windjammer] Error in system: {ex}");
            }
        }

        // Run shutdown systems
        foreach (var system in _shutdownSystems)
        {
            try
            {
                system();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"[Windjammer] Error in shutdown system: {ex}");
            }
        }

        Console.WriteLine("[Windjammer] Application finished");
        _running = false;
    }

    /// <summary>
    /// Checks if the application is currently running.
    /// </summary>
    /// <returns>True if running, false otherwise.</returns>
    public bool IsRunning() => _running;

    /// <summary>
    /// Requests the application to quit.
    /// </summary>
    public void Quit()
    {
        _running = false;
        Console.WriteLine("[Windjammer] Quit requested");
    }
}

