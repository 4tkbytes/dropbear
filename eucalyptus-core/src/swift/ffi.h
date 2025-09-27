// This is the header for the Swift FFI functions. This is just to be used as reference.
//
// I suppose you could use this for any other language other than Swift...

// ----------------------------------
// An expandable vector of uint32_t
typedef struct {
    const uint32_t* ptr;
    size_t len;
    size_t cap;
} CArrayU32;

// Creates a new empty CArrayU32
CArrayU32 new_array_u32(void);

/// Frees memory of a CArrayU32
void free_array_u32(CArrayU32 arr);

// ----------------------------------

// A expandable vector of uint8_t
typedef struct {
    const uint8_t* ptr;
    size_t len;
    size_t cap;
} CArrayU8;

// Creates a new empty CArrayU8
CArrayU8 new_array_u8(void);

/// Frees memory of a CArrayU8
void free_array_u8(CArrayU8 arr);

// ----------------------------------