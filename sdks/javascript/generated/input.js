/** Input manager for keyboard and mouse */
export class Input {
  constructor() {
  }

  /** Returns true if key was just pressed this frame */
  is_key_pressed(key) {
    throw new Error('Not implemented');
  }

  /** Returns true if key is currently held down */
  is_key_held(key) {
    throw new Error('Not implemented');
  }

  /** Returns true if mouse button was just pressed */
  is_mouse_button_pressed(button) {
    throw new Error('Not implemented');
  }

  /** Returns current mouse position */
  mouse_position() {
    throw new Error('Not implemented');
  }

}
