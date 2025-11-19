/**
 * @file app.hpp
 * @brief Main application class
 */

#pragma once

#include <functional>
#include <vector>
#include <iostream>
#include <exception>

namespace wj {

/// System function type
using SystemFunc = std::function<void()>;

/// System function with time parameter
using SystemFuncWithTime = std::function<void(const class Time&)>;

/// Startup system function type
using StartupSystemFunc = std::function<void()>;

/// Shutdown system function type
using ShutdownSystemFunc = std::function<void()>;

/**
 * @brief Main application class for Windjammer games.
 * 
 * @example
 * @code
 * wj::App app;
 * app.add_system([]() { std::cout << "Update!\n"; });
 * app.run();
 * @endcode
 */
class App {
public:
    /**
     * @brief Constructs a new Windjammer application.
     */
    App() {
        std::cout << "[Windjammer] Initializing application...\n";
    }

    /**
     * @brief Adds a system that runs every frame.
     * 
     * @param system The system function to add
     * @return Reference to this app for chaining
     */
    App& add_system(SystemFunc system) {
        systems_.push_back(std::move(system));
        return *this;
    }

    /**
     * @brief Adds a system with time parameter that runs every frame.
     * 
     * @param system The system function to add
     * @return Reference to this app for chaining
     */
    App& add_system(SystemFuncWithTime system) {
        systems_with_time_.push_back(std::move(system));
        return *this;
    }

    /**
     * @brief Adds a startup system that runs once at the beginning.
     * 
     * @param system The startup system function to add
     * @return Reference to this app for chaining
     */
    App& add_startup_system(StartupSystemFunc system) {
        startup_systems_.push_back(std::move(system));
        return *this;
    }

    /**
     * @brief Adds a shutdown system that runs once at the end.
     * 
     * @param system The shutdown system function to add
     * @return Reference to this app for chaining
     */
    App& add_shutdown_system(ShutdownSystemFunc system) {
        shutdown_systems_.push_back(std::move(system));
        return *this;
    }

    /**
     * @brief Runs the application.
     * 
     * This starts the game loop and runs until the application is closed.
     */
    void run() {
        std::cout << "[Windjammer] Starting application with " 
                  << (systems_.size() + systems_with_time_.size()) 
                  << " systems\n";

        // Run startup systems
        for (const auto& system : startup_systems_) {
            try {
                system();
            } catch (const std::exception& e) {
                std::cerr << "[Windjammer] Error in startup system: " << e.what() << "\n";
            }
        }

        running_ = true;

        // TODO: Start actual game loop with FFI
        // For now, just run systems once as a demonstration
        std::cout << "[Windjammer] Running systems...\n";
        
        for (const auto& system : systems_) {
            try {
                system();
            } catch (const std::exception& e) {
                std::cerr << "[Windjammer] Error in system: " << e.what() << "\n";
            }
        }

        // TODO: Create Time instance and pass to systems_with_time_

        // Run shutdown systems
        for (const auto& system : shutdown_systems_) {
            try {
                system();
            } catch (const std::exception& e) {
                std::cerr << "[Windjammer] Error in shutdown system: " << e.what() << "\n";
            }
        }

        std::cout << "[Windjammer] Application finished\n";
        running_ = false;
    }

    /**
     * @brief Checks if the application is currently running.
     * 
     * @return true if running, false otherwise
     */
    [[nodiscard]] bool is_running() const noexcept {
        return running_;
    }

    /**
     * @brief Requests the application to quit.
     */
    void quit() {
        running_ = false;
        std::cout << "[Windjammer] Quit requested\n";
    }

private:
    std::vector<SystemFunc> systems_;
    std::vector<SystemFuncWithTime> systems_with_time_;
    std::vector<StartupSystemFunc> startup_systems_;
    std::vector<ShutdownSystemFunc> shutdown_systems_;
    bool running_ = false;
};

} // namespace wj

