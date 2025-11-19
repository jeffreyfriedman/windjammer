/** Input manager for keyboard and mouse */
export class Input {
  /** Returns true if key was just pressed this frame */
  is_key_pressed(key: Key): boolean {
    throw new Error('Not implemented');
  }

  /** Returns true if key is currently held down */
  is_key_held(key: Key): boolean {
    throw new Error('Not implemented');
  }

  /** Returns true if mouse button was just pressed */
  is_mouse_button_pressed(button: MouseButton): boolean {
    throw new Error('Not implemented');
  }

  /** Returns current mouse position */
  mouse_position(): Vec2 {
    throw new Error('Not implemented');
  }

}
