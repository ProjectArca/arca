#include "../runtime/arca_runtime.h"

void* arca_mem_alloc(size_t size) {
    void* ptr = malloc(size);
    if (!ptr && size > 0) {
        fprintf(stderr, "[arca_runtime] Out of memory allocating %zu bytes\n", size);
        exit(1);
    }
    return ptr;
}

void arca_mem_free(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}
