/** Time information */
export class Time {
  constructor(delta_seconds, total_seconds, frame_count) {
    this.delta_seconds = delta_seconds;
    this.total_seconds = total_seconds;
    this.frame_count = frame_count;
  }
}
