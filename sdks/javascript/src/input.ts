/**
 * Input handling.
 */

/**
 * Keyboard key codes.
 */
export enum KeyCode {
  A = 'A',
  B = 'B',
  C = 'C',
  D = 'D',
  E = 'E',
  F = 'F',
  W = 'W',
  S = 'S',
  Space = 'Space',
  Escape = 'Escape',
}

/**
 * Mouse button codes.
 */
export enum MouseButton {
  Left = 0,
  Right = 1,
  Middle = 2,
}

/**
 * Input state manager.
 */
export class Input {
  private keys = new Set<KeyCode>();
  private mouseButtons = new Set<MouseButton>();

  /**
   * Check if a key is currently pressed.
   * 
   * @param key - The key to check
   * @returns True if pressed, false otherwise
   */
  isKeyPressed(key: KeyCode): boolean {
    return this.keys.has(key);
  }

  /**
   * Check if a mouse button is currently pressed.
   * 
   * @param button - The button to check
   * @returns True if pressed, false otherwise
   */
  isMouseButtonPressed(button: MouseButton): boolean {
    return this.mouseButtons.has(button);
  }
}

