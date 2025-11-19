/** Audio source component */
export class AudioSource {
  volume: number;
  pitch: number;
  looping: boolean;
  /** Plays an audio file */
  play(audio_path: string) {
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
