#ifndef DROPBEAR_H
#define DROPBEAR_H

#include <stddef.h>
#include <stdint.h>

// ===========================================

typedef struct World World; // opaque pointer
typedef struct InputState InputState; // opaque pointer

// ===========================================

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// ===========================================

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

// ===========================================

int dropbear_get_entity(const char* label, const World* world_ptr, int64_t* out_entity);

int dropbear_get_transform(
    const World* world_ptr,
    int64_t entity_id,
    NativeTransform* out_transform
);

int dropbear_set_transform(
    const World* world_ptr,
    int64_t entity_id,
    const NativeTransform transform
);

void dropbear_print_input_state(const InputState* input_state_ptr);

int dropbear_is_key_pressed(const InputState* input_state_ptr, int keycode, int* out_value); // out_value is a boolean 0 or 1

// ===========================================

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
#endif // DROPBEAR_H