#include "../runtime/arca_runtime.h"

typedef struct ArcaArenaChunk {
    struct ArcaArenaChunk* next;
    size_t capacity;
    size_t used;
    uint8_t data[];
} ArcaArenaChunk;

typedef struct {
    ArcaArenaChunk* head;
    size_t default_chunk_size;
} ArcaArena;

void* arca_arena_create(size_t chunk_size) {
    ArcaArena* arena = (ArcaArena*)malloc(sizeof(ArcaArena));
    arena->head = NULL;
    arena->default_chunk_size = chunk_size > 0 ? chunk_size : 4096;
    return arena;
}
