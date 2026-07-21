#include "../runtime/arca_runtime.h"

typedef struct {
    size_t block_size;
    size_t block_count;
    void* memory;
    void* free_list;
} ArcaPool;

void* arca_pool_create(size_t block_size, size_t block_count) {
    ArcaPool* pool = (ArcaPool*)malloc(sizeof(ArcaPool));
    pool->block_size = block_size;
    pool->block_count = block_count;
    pool->memory = malloc(block_size * block_count);
    pool->free_list = NULL;
    return pool;
}
