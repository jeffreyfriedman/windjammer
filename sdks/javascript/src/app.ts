/**
 * Main application class for Windjammer games.
 */

import { Time } from './time';

/**
 * System function type - runs every frame
 */
export type System = (time?: Time) => void;

/**
 * Startup system function type - runs once at startup
 */
export type StartupSystem = () => void;

/**
 * Shutdown system function type - runs once at shutdown
 */
export type ShutdownSystem = () => void;

/**
 * Main application class for Windjammer games.
 * 
 * @example
 * ```typescript
 * const app = new App();
 * app.addSystem(() => console.log('Update!'));
 * app.run();
 * ```
 */
export class App {
  private systems: System[] = [];
  private startupSystems: StartupSystem[] = [];
  private shutdownSystems: ShutdownSystem[] = [];
  private running = false;

  /**
   * Create a new Windjammer application.
   */
  constructor() {
    console.log('[Windjammer] Initializing application...');
  }

  /**
   * Add a system that runs every frame.
   * 
   * @param system - The system function to add
   * @returns This app instance for chaining
   * 
   * @example
   * ```typescript
   * app.addSystem((time) => {
   *   console.log(`Delta: ${time.deltaSeconds}`);
   * });
   * ```
   */
  addSystem(system: System): this {
    this.systems.push(system);
    return this;
  }

  /**
   * Add a startup system that runs once at the beginning.
   * 
   * @param system - The startup system function to add
   * @returns This app instance for chaining
   * 
   * @example
   * ```typescript
   * app.addStartupSystem(() => {
   *   console.log('Setup!');
   * });
   * ```
   */
  addStartupSystem(system: StartupSystem): this {
    this.startupSystems.push(system);
    return this;
  }

  /**
   * Add a shutdown system that runs once at the end.
   * 
   * @param system - The shutdown system function to add
   * @returns This app instance for chaining
   * 
   * @example
   * ```typescript
   * app.addShutdownSystem(() => {
   *   console.log('Cleanup!');
   * });
   * ```
   */
  addShutdownSystem(system: ShutdownSystem): this {
    this.shutdownSystems.push(system);
    return this;
  }

  /**
   * Run the application.
   * 
   * This starts the game loop and runs until the application is closed.
   */
  run(): void {
    console.log(`[Windjammer] Starting application with ${this.systems.length} systems`);

    // Run startup systems
    for (const system of this.startupSystems) {
      try {
        system();
      } catch (error) {
        console.error('[Windjammer] Error in startup system:', error);
      }
    }

    this.running = true;

    // TODO: Start actual game loop with FFI
    // For now, just run systems once as a demonstration
    console.log('[Windjammer] Running systems...');
    const time = new Time();
    for (const system of this.systems) {
      try {
        system(time);
      } catch (error) {
        console.error('[Windjammer] Error in system:', error);
      }
    }

    // Run shutdown systems
    for (const system of this.shutdownSystems) {
      try {
        system();
      } catch (error) {
        console.error('[Windjammer] Error in shutdown system:', error);
      }
    }

    console.log('[Windjammer] Application finished');
    this.running = false;
  }

  /**
   * Check if the application is currently running.
   * 
   * @returns True if running, false otherwise
   */
  isRunning(): boolean {
    return this.running;
  }

  /**
   * Request the application to quit.
   */
  quit(): void {
    this.running = false;
    console.log('[Windjammer] Quit requested');
  }
}

