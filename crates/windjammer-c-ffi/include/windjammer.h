#ifndef WINDJAMMER_H
#define WINDJAMMER_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Behavior tree node types
 */
typedef enum WjBehaviorNodeType {
  /**
   * Sequence node (runs children in order, fails on first failure)
   */
  Sequence = 0,
  /**
   * Selector node (runs children in order, succeeds on first success)
   */
  Selector = 1,
  /**
   * Parallel node (runs all children simultaneously)
   */
  Parallel = 2,
  /**
   * Decorator node (modifies child behavior)
   */
  Decorator = 3,
  /**
   * Action node (leaf node that performs an action)
   */
  Action = 4,
  /**
   * Condition node (leaf node that checks a condition)
   */
  Condition = 5,
} WjBehaviorNodeType;

/**
 * Physics body type
 */
typedef enum WjBodyType {
  /**
   * Dynamic body (affected by forces)
   */
  Dynamic = 0,
  /**
   * Static body (never moves)
   */
  Static = 1,
  /**
   * Kinematic body (moves but not affected by forces)
   */
  Kinematic = 2,
} WjBodyType;

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
 * Network transport protocol
 */
typedef enum WjNetworkProtocol {
  TCP = 0,
  UDP = 1,
} WjNetworkProtocol;

/**
 * Steering behavior types
 */
typedef enum WjSteeringBehavior {
  Seek = 0,
  Flee = 1,
  Arrive = 2,
  Pursue = 3,
  Evade = 4,
  Wander = 5,
} WjSteeringBehavior;

/**
 * Widget types
 */
typedef enum WjWidgetType {
  Button = 0,
  Label = 1,
  Image = 2,
  Slider = 3,
  Checkbox = 4,
  InputField = 5,
} WjWidgetType;

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

/**
 * Raycast result
 */
typedef struct WjRaycastHit2D {
  bool hit;
  struct WjVec2 point;
  struct WjVec2 normal;
  float distance;
  struct WjEntity *entity;
} WjRaycastHit2D;

/**
 * Raycast result (3D)
 */
typedef struct WjRaycastHit3D {
  bool hit;
  struct WjVec3 point;
  struct WjVec3 normal;
  float distance;
  struct WjEntity *entity;
} WjRaycastHit3D;

/**
 * Opaque handle to an audio source
 */
typedef struct WjAudioSource {
  uint8_t _private[0];
} WjAudioSource;

/**
 * Time information
 */
typedef struct WjTime {
  float delta_time;
  float total_time;
  uint64_t frame_count;
  float fps;
} WjTime;

/**
 * Opaque handle to a behavior tree
 */
typedef struct WjBehaviorTree {
  uint8_t _private[0];
} WjBehaviorTree;

/**
 * Path result
 */
typedef struct WjPath {
  struct WjVec3 *points;
  uintptr_t point_count;
} WjPath;

/**
 * Opaque handle to a state machine
 */
typedef struct WjStateMachine {
  uint8_t _private[0];
} WjStateMachine;

/**
 * Opaque handle to a network connection
 */
typedef struct WjNetworkConnection {
  uint8_t _private[0];
} WjNetworkConnection;

/**
 * RPC callback function type
 */
typedef void (*WjRpcCallback)(struct WjEntity *entity, const uint8_t *data, uintptr_t data_len);

/**
 * Network statistics
 */
typedef struct WjNetworkStats {
  uint64_t bytes_sent;
  uint64_t bytes_received;
  uint64_t packets_sent;
  uint64_t packets_received;
  uint64_t packets_lost;
  float ping_ms;
} WjNetworkStats;

/**
 * Opaque handle to an animation clip
 */
typedef struct WjAnimationClip {
  uint8_t _private[0];
} WjAnimationClip;

/**
 * Opaque handle to a UI widget
 */
typedef struct WjWidget {
  uint8_t _private[0];
} WjWidget;

/**
 * UI click callback
 */
typedef void (*WjUiClickCallback)(struct WjWidget *widget);

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
                               struct WjTexture *_texture,
                               struct WjVec2 _position,
                               struct WjVec2 _size,
                               struct WjColor _color);

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
struct WjMesh *wj_mesh_cube(float _size);

/**
 * Create a sphere mesh
 */
struct WjMesh *wj_mesh_sphere(float _radius, unsigned int _subdivisions);

/**
 * Create a plane mesh
 */
struct WjMesh *wj_mesh_plane(float _size);

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
struct WjTexture *wj_texture_from_data(unsigned int _width,
                                       unsigned int _height,
                                       const uint8_t *data,
                                       uintptr_t _data_len);

/**
 * Free a texture
 */
void wj_texture_free(struct WjTexture *texture);

/**
 * Create a 2D camera
 */
enum WjErrorCode wj_camera2d_new(struct WjVec2 _position, float _zoom);

/**
 * Create a 3D camera
 */
enum WjErrorCode wj_camera3d_new(struct WjVec3 _position, struct WjVec3 _look_at, float _fov);

/**
 * Create a point light
 */
enum WjErrorCode wj_point_light_new(struct WjVec3 _position,
                                    struct WjColor _color,
                                    float _intensity);

/**
 * Create a directional light
 */
enum WjErrorCode wj_directional_light_new(struct WjVec3 _direction,
                                          struct WjColor _color,
                                          float _intensity);

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
                                    struct WjVec2 _position,
                                    float _rotation,
                                    struct WjVec2 _scale);

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
                                    struct WjVec3 _position,
                                    struct WjQuat _rotation,
                                    struct WjVec3 _scale);

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
enum WjErrorCode wj_add_velocity2d(struct WjEntity *entity, struct WjVec2 _velocity);

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
bool wj_input_is_gamepad_button_down(int _gamepad_id, enum WjGamepadButton _button);

/**
 * Get gamepad axis value
 */
float wj_input_get_gamepad_axis(int _gamepad_id, enum WjGamepadAxis _axis);

/**
 * Add RigidBody2D component to entity
 */
enum WjErrorCode wj_add_rigidbody2d(struct WjEntity *entity, enum WjBodyType body_type, float mass);

/**
 * Add BoxCollider2D component to entity
 */
enum WjErrorCode wj_add_box_collider2d(struct WjEntity *entity,
                                       struct WjVec2 size,
                                       struct WjVec2 offset);

/**
 * Add CircleCollider2D component to entity
 */
enum WjErrorCode wj_add_circle_collider2d(struct WjEntity *entity,
                                          float radius,
                                          struct WjVec2 offset);

/**
 * Apply force to 2D rigid body
 */
enum WjErrorCode wj_rigidbody2d_apply_force(struct WjEntity *entity, struct WjVec2 force);

/**
 * Apply impulse to 2D rigid body
 */
enum WjErrorCode wj_rigidbody2d_apply_impulse(struct WjEntity *entity, struct WjVec2 impulse);

/**
 * Set 2D rigid body velocity
 */
enum WjErrorCode wj_rigidbody2d_set_velocity(struct WjEntity *entity, struct WjVec2 velocity);

/**
 * Get 2D rigid body velocity
 */
struct WjVec2 wj_rigidbody2d_get_velocity(struct WjEntity *entity);

/**
 * Add RigidBody3D component to entity
 */
enum WjErrorCode wj_add_rigidbody3d(struct WjEntity *entity, enum WjBodyType body_type, float mass);

/**
 * Add BoxCollider3D component to entity
 */
enum WjErrorCode wj_add_box_collider3d(struct WjEntity *entity,
                                       struct WjVec3 size,
                                       struct WjVec3 offset);

/**
 * Add SphereCollider3D component to entity
 */
enum WjErrorCode wj_add_sphere_collider3d(struct WjEntity *entity,
                                          float radius,
                                          struct WjVec3 offset);

/**
 * Add CapsuleCollider3D component to entity
 */
enum WjErrorCode wj_add_capsule_collider3d(struct WjEntity *entity,
                                           float radius,
                                           float height,
                                           struct WjVec3 offset);

/**
 * Apply force to 3D rigid body
 */
enum WjErrorCode wj_rigidbody3d_apply_force(struct WjEntity *entity, struct WjVec3 force);

/**
 * Apply torque to 3D rigid body
 */
enum WjErrorCode wj_rigidbody3d_apply_torque(struct WjEntity *entity, struct WjVec3 torque);

/**
 * Perform 2D raycast
 */
struct WjRaycastHit2D wj_raycast2d(struct WjWorld *world,
                                   struct WjVec2 origin,
                                   struct WjVec2 direction,
                                   float max_distance);

/**
 * Perform 3D raycast
 */
struct WjRaycastHit3D wj_raycast3d(struct WjWorld *world,
                                   struct WjVec3 origin,
                                   struct WjVec3 direction,
                                   float max_distance);

/**
 * Load an audio file
 */
struct WjAudioSource *wj_audio_load(const char *path);

/**
 * Free an audio source
 */
void wj_audio_free(struct WjAudioSource *source);

/**
 * Play an audio source
 */
enum WjErrorCode wj_audio_play(struct WjAudioSource *source);

/**
 * Stop an audio source
 */
enum WjErrorCode wj_audio_stop(struct WjAudioSource *source);

/**
 * Pause an audio source
 */
enum WjErrorCode wj_audio_pause(struct WjAudioSource *source);

/**
 * Resume an audio source
 */
enum WjErrorCode wj_audio_resume(struct WjAudioSource *source);

/**
 * Set audio volume (0.0 to 1.0)
 */
enum WjErrorCode wj_audio_set_volume(struct WjAudioSource *source, float volume);

/**
 * Set audio pitch (0.5 to 2.0, 1.0 is normal)
 */
enum WjErrorCode wj_audio_set_pitch(struct WjAudioSource *source, float pitch);

/**
 * Set audio looping
 */
enum WjErrorCode wj_audio_set_looping(struct WjAudioSource *source, bool looping);

/**
 * Set 3D audio position
 */
enum WjErrorCode wj_audio_set_position(struct WjAudioSource *source, struct WjVec3 position);

/**
 * Set 3D audio listener position
 */
enum WjErrorCode wj_audio_set_listener_position(struct WjVec3 position);

/**
 * Set 3D audio listener orientation
 */
enum WjErrorCode wj_audio_set_listener_orientation(struct WjVec3 forward, struct WjVec3 up);

/**
 * Set audio attenuation (how quickly sound fades with distance)
 */
enum WjErrorCode wj_audio_set_attenuation(struct WjAudioSource *source, float attenuation);

/**
 * Set audio min/max distance for 3D audio
 */
enum WjErrorCode wj_audio_set_distance_range(struct WjAudioSource *source,
                                             float min_distance,
                                             float max_distance);

/**
 * Check if audio is playing
 */
bool wj_audio_is_playing(struct WjAudioSource *source);

/**
 * Get audio playback position (in seconds)
 */
float wj_audio_get_playback_position(struct WjAudioSource *source);

/**
 * Get audio duration (in seconds)
 */
float wj_audio_get_duration(struct WjAudioSource *source);

/**
 * Create a new world
 */
struct WjWorld *wj_world_new(void);

/**
 * Free a world
 */
void wj_world_free(struct WjWorld *world);

/**
 * Update world (run systems for one frame)
 */
enum WjErrorCode wj_world_update(struct WjWorld *world, float delta_time);

/**
 * Get number of entities in world
 */
uintptr_t wj_world_entity_count(struct WjWorld *world);

/**
 * Find entity by name
 */
struct WjEntity *wj_world_find_entity(struct WjWorld *world, const char *name);

/**
 * Destroy entity
 */
enum WjErrorCode wj_world_destroy_entity(struct WjWorld *world, struct WjEntity *entity);

/**
 * Save world to file
 */
enum WjErrorCode wj_world_save(struct WjWorld *world, const char *path);

/**
 * Load world from file
 */
struct WjWorld *wj_world_load(const char *path);

/**
 * Clear all entities from world
 */
enum WjErrorCode wj_world_clear(struct WjWorld *world);

/**
 * Get current time information
 */
struct WjTime wj_get_time(void);

/**
 * Set target FPS
 */
enum WjErrorCode wj_set_target_fps(float fps);

/**
 * Set time scale (for slow motion / fast forward)
 */
enum WjErrorCode wj_set_time_scale(float scale);

/**
 * Create a new behavior tree
 */
struct WjBehaviorTree *wj_behavior_tree_new(void);

/**
 * Free a behavior tree
 */
void wj_behavior_tree_free(struct WjBehaviorTree *tree);

/**
 * Add node to behavior tree
 */
enum WjErrorCode wj_behavior_tree_add_node(struct WjBehaviorTree *tree,
                                           enum WjBehaviorNodeType node_type,
                                           const char *name);

/**
 * Tick behavior tree (update for one frame)
 */
enum WjErrorCode wj_behavior_tree_tick(struct WjBehaviorTree *tree,
                                       struct WjEntity *entity,
                                       float delta_time);

/**
 * Find path from start to end
 */
struct WjPath wj_pathfinding_find_path(struct WjWorld *world,
                                       struct WjVec3 start,
                                       struct WjVec3 end);

/**
 * Free path
 */
void wj_path_free(struct WjPath path);

/**
 * Calculate steering force
 */
struct WjVec3 wj_steering_calculate(enum WjSteeringBehavior behavior,
                                    struct WjVec3 position,
                                    struct WjVec3 velocity,
                                    struct WjVec3 target,
                                    float max_speed);

/**
 * Add steering behavior to entity
 */
enum WjErrorCode wj_add_steering_behavior(struct WjEntity *entity,
                                          enum WjSteeringBehavior behavior,
                                          struct WjVec3 target);

/**
 * Create a new state machine
 */
struct WjStateMachine *wj_state_machine_new(void);

/**
 * Free a state machine
 */
void wj_state_machine_free(struct WjStateMachine *sm);

/**
 * Add state to state machine
 */
enum WjErrorCode wj_state_machine_add_state(struct WjStateMachine *sm, const char *state_name);

/**
 * Add transition to state machine
 */
enum WjErrorCode wj_state_machine_add_transition(struct WjStateMachine *sm,
                                                 const char *from_state,
                                                 const char *to_state,
                                                 const char *condition);

/**
 * Update state machine
 */
enum WjErrorCode wj_state_machine_update(struct WjStateMachine *sm,
                                         struct WjEntity *entity,
                                         float delta_time);

/**
 * Get current state
 */
const char *wj_state_machine_get_current_state(struct WjStateMachine *sm);

/**
 * Create a server
 */
struct WjNetworkConnection *wj_network_create_server(unsigned short port,
                                                     enum WjNetworkProtocol protocol);

/**
 * Connect to a server
 */
struct WjNetworkConnection *wj_network_connect(const char *host,
                                               unsigned short port,
                                               enum WjNetworkProtocol protocol);

/**
 * Disconnect
 */
enum WjErrorCode wj_network_disconnect(struct WjNetworkConnection *conn);

/**
 * Free network connection
 */
void wj_network_free(struct WjNetworkConnection *conn);

/**
 * Check if connected
 */
bool wj_network_is_connected(struct WjNetworkConnection *conn);

/**
 * Send message (raw bytes)
 */
enum WjErrorCode wj_network_send(struct WjNetworkConnection *conn,
                                 const uint8_t *data,
                                 uintptr_t data_len,
                                 bool reliable);

/**
 * Receive message (raw bytes)
 */
enum WjErrorCode wj_network_receive(struct WjNetworkConnection *conn,
                                    uint8_t *buffer,
                                    uintptr_t buffer_size,
                                    uintptr_t *bytes_received);

/**
 * Mark entity for replication
 */
enum WjErrorCode wj_network_replicate_entity(struct WjNetworkConnection *conn,
                                             struct WjEntity *entity);

/**
 * Stop replicating entity
 */
enum WjErrorCode wj_network_stop_replicating_entity(struct WjNetworkConnection *conn,
                                                    struct WjEntity *entity);

/**
 * Register RPC handler
 */
enum WjErrorCode wj_network_register_rpc(struct WjNetworkConnection *conn,
                                         const char *rpc_name,
                                         WjRpcCallback callback);

/**
 * Call RPC
 */
enum WjErrorCode wj_network_call_rpc(struct WjNetworkConnection *conn,
                                     const char *rpc_name,
                                     struct WjEntity *entity,
                                     const uint8_t *data,
                                     uintptr_t data_len);

/**
 * Get network statistics
 */
struct WjNetworkStats wj_network_get_stats(struct WjNetworkConnection *conn);

/**
 * Load animation clip
 */
struct WjAnimationClip *wj_animation_load(const char *path);

/**
 * Free animation clip
 */
void wj_animation_free(struct WjAnimationClip *clip);

/**
 * Play animation
 */
enum WjErrorCode wj_animation_play(struct WjEntity *entity,
                                   struct WjAnimationClip *clip,
                                   bool loop_animation);

/**
 * Stop animation
 */
enum WjErrorCode wj_animation_stop(struct WjEntity *entity);

/**
 * Set animation speed
 */
enum WjErrorCode wj_animation_set_speed(struct WjEntity *entity, float speed);

/**
 * Blend between two animations
 */
enum WjErrorCode wj_animation_blend(struct WjEntity *entity,
                                    struct WjAnimationClip *clip_a,
                                    struct WjAnimationClip *clip_b,
                                    float blend_factor);

/**
 * Create UI widget
 */
struct WjWidget *wj_ui_create_widget(enum WjWidgetType widget_type,
                                     struct WjVec2 position,
                                     struct WjVec2 size);

/**
 * Free UI widget
 */
void wj_ui_free_widget(struct WjWidget *widget);

/**
 * Set widget text
 */
enum WjErrorCode wj_ui_set_text(struct WjWidget *widget, const char *text);

/**
 * Set click callback
 */
enum WjErrorCode wj_ui_set_click_callback(struct WjWidget *widget, WjUiClickCallback callback);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* WINDJAMMER_H */
