#include "../runtime/arca_runtime.h"

void* arca_fs_open(const char* path, const char* mode) {
    if (!path || !mode) return NULL;
    return (void*)fopen(path, mode);
}

int32_t arca_fs_close(void* handle) {
    if (!handle) return -1;
    return (int32_t)fclose((FILE*)handle);
}
