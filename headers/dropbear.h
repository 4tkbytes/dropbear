#ifndef DROPBEAR_H
#define DROPBEAR_H

#include <stddef.h>
#include <stdint.h>

// ===========================================

typedef struct World World; // opaque pointer
typedef struct InputState InputState; // opaque pointer
typedef struct GraphicsCommandQueue GraphicsCommandQueue; // opaque pointer
typedef struct AssetRegistry AssetRegistry; // opaque pointer

// ===========================================

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// ===========================================

typedef struct {
    float x;
    float y;
    float z;
} Vector3D;

typedef struct {
    double position_x;
    double position_y;
    double position_z;
    double rotation_x;
    double rotation_y;
    double rotation_z;
    double rotation_w;
    double scale_x;
    double scale_y;
    double scale_z;
} NativeTransform;

typedef struct {
    NativeTransform local;
    NativeTransform world;
} NativeEntityTransform;

typedef struct {
    const char* label;
    int64_t entity_id;

    Vector3D eye;
    Vector3D target;
    Vector3D up;

    double aspect;
    double fov_y;
    double znear;
    double zfar;

    double yaw;
    double pitch;
    double speed;
    double sensitivity;
} NativeCamera;

// ===========================================

int dropbear_get_entity(const char* label, const World* world_ptr, int64_t* out_entity);

int dropbear_get_transform(
    const World* world_ptr,
    int64_t entity_id,
    NativeEntityTransform* out_transform
);

int dropbear_set_transform(
    const World* world_ptr,
    int64_t entity_id,
    const NativeEntityTransform transform
);

// property management
int dropbear_get_string_property(const World* world_ptr, int64_t entity_handle, const char* label, char* out_value, int out_value_max_length);
int dropbear_get_int_property(const World* world_ptr, int64_t entity_handle, const char* label, int* out_value);
int dropbear_get_long_property(const World* world_ptr, int64_t entity_handle, const char* label, int64_t* out_value);
int dropbear_get_float_property(const World* world_ptr, int64_t entity_handle, const char* label, float* out_value);
int dropbear_get_double_property(const World* world_ptr, int64_t entity_handle, const char* label, double* out_value);
int dropbear_get_bool_property(const World* world_ptr, int64_t entity_handle, const char* label, int* out_value); // out_value = 0 or 1
int dropbear_get_vec3_property(const World* world_ptr, int64_t entity_handle, const char* label, float* out_x, float* out_y, float* out_z);

int dropbear_set_string_property(const World* world_ptr, int64_t entity_handle, const char* label, const char* value);
int dropbear_set_int_property(const World* world_ptr, int64_t entity_handle, const char* label, int value);
int dropbear_set_long_property(const World* world_ptr, int64_t entity_handle, const char* label, int64_t value);
int dropbear_set_float_property(const World* world_ptr, int64_t entity_handle, const char* label, float value);
int dropbear_set_double_property(const World* world_ptr, int64_t entity_handle, const char* label, double value);
int dropbear_set_bool_property(const World* world_ptr, int64_t entity_handle, const char* label, int value); // value = 0 or 1
int dropbear_set_vec3_property(const World* world_ptr, int64_t entity_handle, const char* label, float x, float y, float z);


// input stuff
void dropbear_print_input_state(const InputState* input_state_ptr);
int dropbear_is_key_pressed(const InputState* input_state_ptr, int keycode, int* out_value); // out_value = 0 or 1
int dropbear_get_mouse_position(const InputState* input_state_ptr, float* out_x, float* out_y);
int dropbear_is_mouse_button_pressed(const InputState* input_state_ptr, int button_code, int* out_pressed);
int dropbear_get_mouse_delta(const InputState* input_state_ptr, float* out_delta_x, float* out_delta_y);
int dropbear_is_cursor_locked(const InputState* input_state_ptr, int* out_locked);
int dropbear_set_cursor_locked(const GraphicsCommandQueue* graphics_ptr, const InputState* input_state_ptr, int locked);
int dropbear_get_last_mouse_pos(const InputState* input_state_ptr, float* out_x, float* out_y);
int dropbear_is_cursor_hidden(const InputState* input_state_ptr, int* out_hidden);
int dropbear_set_cursor_hidden(const GraphicsCommandQueue* graphics_ptr, const InputState* input_state_ptr, int hidden);

// camera
int dropbear_get_camera(const World* world_ptr, const char* label, NativeCamera* out_camera);
int dropbear_get_attached_camera(const World* world_ptr, int64_t id, NativeCamera* out_camera);
int dropbear_set_camera(const World* world_ptr, const NativeCamera* camera);

// ===========================================

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
#endif // DROPBEAR_H
