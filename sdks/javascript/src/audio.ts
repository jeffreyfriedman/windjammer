/**
 * Audio components.
 */

import { Vec3 } from './math';

/**
 * Options for creating an audio source.
 */
export interface AudioSourceOptions {
  /** Audio file path */
  audioFile?: string;
  /** Volume level */
  volume?: number;
  /** Whether to loop */
  looping?: boolean;
}

/**
 * Audio source component.
 */
export class AudioSource {
  /** Audio file path */
  audioFile: string;
  
  /** Volume level */
  volume: number;
  
  /** Whether to loop */
  looping: boolean;
  
  /** Position in 3D space */
  position: Vec3;

  /**
   * Create a new audio source.
   * 
   * @param options - Audio source options
   */
  constructor(options: AudioSourceOptions = {}) {
    this.audioFile = options.audioFile || '';
    this.volume = options.volume ?? 1.0;
    this.looping = options.looping ?? false;
    this.position = Vec3.zero();
  }

  /**
   * Play the audio.
   */
  play(): void {
    console.log(`[Audio] Playing: ${this.audioFile}`);
  }

  /**
   * Stop the audio.
   */
  stop(): void {
    console.log(`[Audio] Stopping: ${this.audioFile}`);
  }

  toString(): string {
    return `AudioSource(file='${this.audioFile}', volume=${this.volume})`;
  }
}

/**
 * Audio listener component.
 */
export class AudioListener {
  /** Position in 3D space */
  position: Vec3;

  /**
   * Create a new audio listener.
   * 
   * @param position - Listener position
   */
  constructor(position: Vec3 = Vec3.zero()) {
    this.position = position;
  }

  toString(): string {
    return `AudioListener(pos=${this.position})`;
  }
}

