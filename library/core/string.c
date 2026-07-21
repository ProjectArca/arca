#include "../runtime/arca_runtime.h"

int64_t arca_str_len(const char* str) {
    return str ? (int64_t)strlen(str) : 0;
}

int32_t arca_str_cmp(const char* s1, const char* s2) {
    if (!s1 && !s2) return 0;
    if (!s1) return -1;
    if (!s2) return 1;
    return (int32_t)strcmp(s1, s2);
}
