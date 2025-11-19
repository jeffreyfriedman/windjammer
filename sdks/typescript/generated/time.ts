/** Time information */
export interface Time {
  /** Time since last frame */
  delta_seconds: number;
  /** Total time since start */
  total_seconds: number;
  /** Frame number */
  frame_count: number;
}
