#include "arca_runtime.h"

void arca_print_int(int64_t v) {
    printf("%lld", (long long)v);
}

void arca_print_string(const char* s) {
    if (s) {
        fwrite(s, strlen(s), 1, stdout);
    }
}

void arca_println_int(int64_t v) {
    printf("%lld\n", (long long)v);
}

void arca_println_string(const char* s) {
    if (s) {
        puts(s);
    } else {
        putchar('\n');
    }
}

int64_t arca_time_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
}

int64_t arca_time_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000LL + ((int64_t)ts.tv_nsec / 1000000LL);
}

// String helpers for CRUD operations
int32_t arca_strcmp(const char* a, const char* b) {
    if (!a || !b) return a == b ? 0 : -1;
    return strcmp(a, b);
}

int32_t arca_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return 0;
    return strncmp(s, prefix, strlen(prefix)) == 0 ? 1 : 0;
}

const char* arca_strcat(const char* a, const char* b) {
    if (!a) a = "";
    if (!b) b = "";
    size_t alen = strlen(a);
    size_t blen = strlen(b);
    char* result = (char*)malloc(alen + blen + 1);
    if (!result) return a;
    memcpy(result, a, alen);
    memcpy(result + alen, b, blen + 1);
    return result;
}

int32_t arca_parse_int(const char* s) {
    if (!s) return 0;
    return (int32_t)atoi(s);
}

const char* arca_int_to_str(int32_t n) {
    static char buf[32];
    snprintf(buf, sizeof(buf), "%d", n);
    return buf;
}

int32_t arca_str_rfind(const char* s, char c) {
    if (!s) return -1;
    const char* p = strrchr(s, c);
    return p ? (int32_t)(p - s) : -1;
}

const char* arca_str_slice(const char* s, int32_t start) {
    if (!s) return "";
    size_t len = strlen(s);
    if (start < 0 || (size_t)start >= len) return "";
    return s + start;
}
