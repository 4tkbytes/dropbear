#ifndef DROPBEAR_H
#define DROPBEAR_H

#include <stddef.h>
#include <stdint.h>

typedef struct World World; // opaque pointer

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// ===========================================

// returns 0 on success, non-zero on failure
int dropbear_get_entity(const char* label, const World* world_ptr, int64_t* out_entity);

// ===========================================

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
#endif // DROPBEAR_H