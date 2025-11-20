#ifndef WINDJAMMER_H
#define WINDJAMMER_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Error codes returned by FFI functions
 */
typedef enum WjErrorCode {
  /**
   * Success
   */
  Ok = 0,
  /**
   * Null pointer passed where non-null expected
   */
  NullPointer = 1,
  /**
   * Invalid handle
   */
  InvalidHandle = 2,
  /**
   * Out of memory
   */
  OutOfMemory = 3,
  /**
   * Invalid argument
   */
  InvalidArgument = 4,
  /**
   * Operation failed
   */
  OperationFailed = 5,
  /**
   * Panic occurred
   */
  Panic = 6,
} WjErrorCode;

/**
 * Gamepad axes
 */
typedef enum WjGamepadAxis {
  LeftStickX = 0,
  LeftStickY,
  RightStickX,
  RightStickY,
  LeftTrigger,
  RightTrigger,
} WjGamepadAxis;

/**
 * Gamepad buttons
 */
typedef enum WjGamepadButton {
  A = 0,
  B,
  X,
  Y,
  LeftBumper,
  RightBumper,
  Back,
  Start,
  LeftStick,
  RightStick,
  DPadUp,
  DPadDown,
  DPadLeft,
  DPadRight,
} WjGamepadButton;

/**
 * Key codes (subset of common keys)
 */
typedef enum WjKeyCode {
  A = 0,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
  K,
  L,
  M,
  N,
  O,
  P,
  Q,
  R,
  S,
  T,
  U,
  V,
  W,
  X,
  Y,
  Z,
  Key0 = 100,
  Key1,
  Key2,
  Key3,
  Key4,
  Key5,
  Key6,
  Key7,
  Key8,
  Key9,
  F1 = 200,
  F2,
  F3,
  F4,
  F5,
  F6,
  F7,
  F8,
  F9,
  F10,
  F11,
  F12,
  Space = 300,
  Enter,
  Escape,
  Tab,
  Backspace,
  Delete,
  Left = 400,
  Right,
  Up,
  Down,
  LeftShift = 500,
  RightShift,
  LeftControl,
  RightControl,
  LeftAlt,
  RightAlt,
} WjKeyCode;

/**
 * Mouse buttons
 */
typedef enum WjMouseButton {
  Left = 0,
  Right = 1,
  Middle = 2,
} WjMouseButton;

/**
 * Opaque handle to the game engine
 */
typedef struct WjEngine {
  uint8_t _private[0];
} WjEngine;

/**
 * Opaque handle to a window
 */
typedef struct WjWindow {
  uint8_t _private[0];
} WjWindow;

/**
 * Opaque handle to an entity
 */
typedef struct WjEntity {
  uint8_t _private[0];
} WjEntity;

/**
 * Opaque handle to a world
 */
typedef struct WjWorld {
  uint8_t _private[0];
} WjWorld;

/**
 * 2D vector
 */
typedef struct WjVec2 {
  float x;
  float y;
} WjVec2;

/**
 * 3D vector
 */
typedef struct WjVec3 {
  float x;
  float y;
  float z;
} WjVec3;

/**
 * 4D vector
 */
typedef struct WjVec4 {
  float x;
  float y;
  float z;
  float w;
} WjVec4;

/**
 * Color (RGBA)
 */
typedef struct WjColor {
  float r;
  float g;
  float b;
  float a;
} WjColor;

/**
 * Opaque handle to a texture
 */
typedef struct WjTexture {
  uint8_t _private[0];
} WjTexture;

/**
 * Opaque handle to a mesh
 */
typedef struct WjMesh {
  uint8_t _private[0];
} WjMesh;

/**
 * Material properties
 */
typedef struct WjMaterial {
  struct WjColor albedo;
  float metallic;
  float roughness;
  struct WjColor emissive;
} WjMaterial;

/**
 * Quaternion
 */
typedef struct WjQuat {
  float x;
  float y;
  float z;
  float w;
} WjQuat;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Get the last error message
 */
const char *wj_get_last_error(void);

/**
 * Clear the last error
 */
void wj_clear_last_error(void);

/**
 * Allocate memory
 */
void *wj_malloc(uintptr_t size);

/**
 * Free memory
 */
void wj_free(void *ptr);

/**
 * Create a new C string
 */
char *wj_string_new(const char *s);

/**
 * Free a C string
 */
void wj_string_free(char *s);

/**
 * Create a new engine instance
 */
struct WjEngine *wj_engine_new(void);

/**
 * Destroy an engine instance
 */
void wj_engine_free(struct WjEngine *engine);

/**
 * Run the engine (blocking)
 */
enum WjErrorCode wj_engine_run(struct WjEngine *engine);

/**
 * Create a new window
 */
struct WjWindow *wj_window_new(const char *title, unsigned int _width, unsigned int _height);

/**
 * Destroy a window
 */
void wj_window_free(struct WjWindow *window);

/**
 * Create a new entity
 */
struct WjEntity *wj_entity_new(struct WjWorld *world);

/**
 * Destroy an entity
 */
void wj_entity_free(struct WjEntity *entity);

/**
 * Create a Vec2
 */
struct WjVec2 wj_vec2_new(float x, float y);

/**
 * Create a Vec3
 */
struct WjVec3 wj_vec3_new(float x, float y, float z);

/**
 * Create a Vec4
 */
struct WjVec4 wj_vec4_new(float x, float y, float z, float w);

/**
 * Create a Color
 */
struct WjColor wj_color_new(float r, float g, float b, float a);

/**
 * Get the library version
 */
const char *wj_version(void);

/**
 * Get the library version as integers
 */
void wj_version_numbers(int *major, int *minor, int *patch);

/**
 * Create a sprite
 */
enum WjErrorCode wj_sprite_new(struct WjEntity *entity,
                               struct WjTexture *texture,
                               struct WjVec2 position,
                               struct WjVec2 size,
                               struct WjColor color);

/**
 * Set sprite texture
 */
enum WjErrorCode wj_sprite_set_texture(struct WjEntity *entity, struct WjTexture *texture);

/**
 * Set sprite color
 */
enum WjErrorCode wj_sprite_set_color(struct WjEntity *entity, struct WjColor color);

/**
 * Create a cube mesh
 */
struct WjMesh *wj_mesh_cube(float size);

/**
 * Create a sphere mesh
 */
struct WjMesh *wj_mesh_sphere(float radius, unsigned int subdivisions);

/**
 * Create a plane mesh
 */
struct WjMesh *wj_mesh_plane(float size);

/**
 * Free a mesh
 */
void wj_mesh_free(struct WjMesh *mesh);

/**
 * Load a texture from file
 */
struct WjTexture *wj_texture_load(const char *path);

/**
 * Create a texture from raw data
 */
struct WjTexture *wj_texture_from_data(unsigned int width,
                                       unsigned int height,
                                       const uint8_t *data,
                                       uintptr_t data_len);

/**
 * Free a texture
 */
void wj_texture_free(struct WjTexture *texture);

/**
 * Create a 2D camera
 */
enum WjErrorCode wj_camera2d_new(struct WjVec2 position, float zoom);

/**
 * Create a 3D camera
 */
enum WjErrorCode wj_camera3d_new(struct WjVec3 position, struct WjVec3 look_at, float fov);

/**
 * Create a point light
 */
enum WjErrorCode wj_point_light_new(struct WjVec3 position, struct WjColor color, float intensity);

/**
 * Create a directional light
 */
enum WjErrorCode wj_directional_light_new(struct WjVec3 direction,
                                          struct WjColor color,
                                          float intensity);

/**
 * Create a material
 */
struct WjMaterial wj_material_new(struct WjColor albedo, float metallic, float roughness);

/**
 * Set material emissive color
 */
void wj_material_set_emissive(struct WjMaterial *material, struct WjColor emissive);

/**
 * Add Transform2D component to entity
 */
enum WjErrorCode wj_add_transform2d(struct WjEntity *entity,
                                    struct WjVec2 position,
                                    float rotation,
                                    struct WjVec2 scale);

/**
 * Get Transform2D position
 */
struct WjVec2 wj_get_transform2d_position(struct WjEntity *entity);

/**
 * Set Transform2D position
 */
enum WjErrorCode wj_set_transform2d_position(struct WjEntity *entity, struct WjVec2 position);

/**
 * Add Transform3D component to entity
 */
enum WjErrorCode wj_add_transform3d(struct WjEntity *entity,
                                    struct WjVec3 position,
                                    struct WjQuat rotation,
                                    struct WjVec3 scale);

/**
 * Get Transform3D position
 */
struct WjVec3 wj_get_transform3d_position(struct WjEntity *entity);

/**
 * Set Transform3D position
 */
enum WjErrorCode wj_set_transform3d_position(struct WjEntity *entity, struct WjVec3 position);

/**
 * Add Velocity2D component to entity
 */
enum WjErrorCode wj_add_velocity2d(struct WjEntity *entity, struct WjVec2 velocity);

/**
 * Get Velocity2D
 */
struct WjVec2 wj_get_velocity2d(struct WjEntity *entity);

/**
 * Set Velocity2D
 */
enum WjErrorCode wj_set_velocity2d(struct WjEntity *entity, struct WjVec2 velocity);

/**
 * Add Name component to entity
 */
enum WjErrorCode wj_add_name(struct WjEntity *entity, const char *name);

/**
 * Get Name component
 */
const char *wj_get_name(struct WjEntity *entity);

/**
 * Check if a key is currently pressed
 */
bool wj_input_is_key_down(enum WjKeyCode key);

/**
 * Check if a key was just pressed this frame
 */
bool wj_input_is_key_pressed(enum WjKeyCode key);

/**
 * Check if a key was just released this frame
 */
bool wj_input_is_key_released(enum WjKeyCode key);

/**
 * Check if a mouse button is currently pressed
 */
bool wj_input_is_mouse_button_down(enum WjMouseButton button);

/**
 * Check if a mouse button was just pressed this frame
 */
bool wj_input_is_mouse_button_pressed(enum WjMouseButton button);

/**
 * Get mouse position
 */
struct WjVec2 wj_input_get_mouse_position(void);

/**
 * Get mouse delta (movement since last frame)
 */
struct WjVec2 wj_input_get_mouse_delta(void);

/**
 * Get mouse scroll delta
 */
struct WjVec2 wj_input_get_mouse_scroll(void);

/**
 * Check if a gamepad button is pressed
 */
bool wj_input_is_gamepad_button_down(int gamepad_id, enum WjGamepadButton button);

/**
 * Get gamepad axis value
 */
float wj_input_get_gamepad_axis(int gamepad_id, enum WjGamepadAxis axis);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* WINDJAMMER_H */
