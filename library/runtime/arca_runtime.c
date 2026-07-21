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
