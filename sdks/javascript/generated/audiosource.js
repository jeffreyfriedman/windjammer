/** Audio source component */
export class AudioSource {
  constructor() {
    this.volume = null;
    this.pitch = null;
    this.looping = null;
  }

  /** Plays an audio file */
  play(audio_path) {
    throw new Error('Not implemented');
  }

  /** Stops playback */
  stop() {
    throw new Error('Not implemented');
  }

  /** Pauses playback */
  pause() {
    throw new Error('Not implemented');
  }

}
