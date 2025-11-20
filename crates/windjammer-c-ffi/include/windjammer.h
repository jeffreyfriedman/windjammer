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

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* WINDJAMMER_H */
