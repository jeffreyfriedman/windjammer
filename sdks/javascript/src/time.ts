/**
 * Time utilities.
 */

/**
 * Time information for the current frame.
 */
export class Time {
  /** Time since last frame in seconds */
  deltaSeconds: number = 0.016; // ~60 FPS
  
  /** Total time since application start in seconds */
  totalSeconds: number = 0.0;
  
  /** Current frame number */
  frameCount: number = 0;

  toString(): string {
    return `Time(delta=${this.deltaSeconds}, total=${this.totalSeconds})`;
  }
}

