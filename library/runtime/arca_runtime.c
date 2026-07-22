#include "arca_runtime.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <time.h>

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

static _Thread_local int64_t g_arca_last_error = 0;

void __arca_throw(int64_t err) {
    g_arca_last_error = err != 0 ? err : -1;
}

int64_t __arca_get_last_error(void) {
    return g_arca_last_error;
}

void __arca_clear_last_error(void) {
    g_arca_last_error = 0;
}

typedef struct {
    int32_t tag;
    int64_t val;
} ArcaResult;

int64_t arca_result_ok(int64_t val) {
    ArcaResult* r = (ArcaResult*)malloc(sizeof(ArcaResult));
    r->tag = 0;
    r->val = val;
    return (int64_t)r;
}

int64_t arca_result_err(int64_t err) {
    ArcaResult* r = (ArcaResult*)malloc(sizeof(ArcaResult));
    r->tag = 1;
    r->val = err;
    return (int64_t)r;
}

int32_t arca_result_is_ok(int64_t res) {
    if (!res) return 0;
    ArcaResult* r = (ArcaResult*)res;
    return r->tag == 0 ? 1 : 0;
}

int64_t arca_result_unwrap(int64_t res) {
    if (!res) return 0;
    ArcaResult* r = (ArcaResult*)res;
    return r->val;
}

int64_t arca_option_some(int64_t val) {
    return arca_result_ok(val);
}

int32_t arca_option_is_some(int64_t opt) {
    return arca_result_is_ok(opt);
}

// std/string: trim leading whitespace
const char* arca_str_trim(const char* s) {
    if (!s) return "";
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    return s;
}

int32_t arca_str_contains(const char* s, const char* sub) {
    if (!s || !sub) return 0;
    return strstr(s, sub) != NULL ? 1 : 0;
}

int32_t arca_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return 0;
    size_t slen = strlen(s);
    size_t suflen = strlen(suffix);
    if (suflen > slen) return 0;
    return strcmp(s + slen - suflen, suffix) == 0 ? 1 : 0;
}

// std/time
void arca_sleep_ms(int64_t ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
}

// std/env
int64_t arca_env_get(const char* name) {
    if (!name) return 0;
    const char* val = getenv(name);
    return (int64_t)val;
}

int64_t arca_env_set(const char* name, const char* value) {
    if (!name || !value) return -1;
    return (int64_t)(setenv(name, value, 1) == 0 ? 0 : -1);
}

int64_t arca_current_dir(void) {
    char* cwd = getcwd(NULL, 0);
    if (!cwd) return 0;
    int64_t ptr = (int64_t)cwd;
    return ptr;
}

// std/io: stdin read line
const char* arca_stdin_read_line(void) {
    static char buf[4096];
    if (!fgets(buf, sizeof(buf), stdin)) return "";
    size_t len = strlen(buf);
    if (len > 0 && buf[len-1] == '\n') buf[len-1] = 0;
    return buf;
}

// std/fs: file operations
int64_t arca_fs_open(const char* path, const char* mode) {
    FILE* f = fopen(path, mode);
    return (int64_t)f;
}

int32_t arca_fs_close(int64_t handle) {
    if (!handle) return -1;
    return fclose((FILE*)handle);
}

int32_t arca_fs_exists(const char* path) {
    if (!path) return 0;
    return access(path, F_OK) == 0 ? 1 : 0;
}

int32_t arca_fs_remove(const char* path) {
    if (!path) return -1;
    return remove(path) == 0 ? 0 : -1;
}

// std/path
const char* arca_path_extension(const char* path) {
    if (!path) return "";
    const char* dot = strrchr(path, '.');
    if (!dot) return "";
    return dot;
}

const char* arca_path_filename(const char* path) {
    if (!path) return "";
    const char* slash = strrchr(path, '/');
    return slash ? slash + 1 : path;
}

const char* arca_path_parent(const char* path) {
    if (!path || !*path) return "";
    static char buf[4096];
    strncpy(buf, path, sizeof(buf) - 1);
    buf[sizeof(buf) - 1] = 0;
    char* slash = strrchr(buf, '/');
    if (!slash) return "";
    *slash = 0;
    return buf;
}

const char* arca_path_join(const char* a, const char* b) {
    if (!a) a = "";
    if (!b) b = "";
    static char buf[4096];
    size_t alen = strlen(a);
    int has_slash = alen > 0 && a[alen-1] == '/';
    snprintf(buf, sizeof(buf), "%s%s%s", a, has_slash ? "" : "/", b);
    return buf;
}

void arca_exit(int64_t code) {
    exit((int)code);
}

const char* arca_json_stringify(const char* s) {
    if (!s) return "\"\"";
    static char buf[8192]; *buf = 0;
    char* w = buf; *w++ = '"';
    while (*s && (size_t)(w - buf) < sizeof(buf) - 4) {
        if (*s == '"' || *s == '\\') { *w++ = '\\'; *w++ = *s; }
        else if (*s == '\n') { *w++ = '\\'; *w++ = 'n'; }
        else if (*s == '\t') { *w++ = '\\'; *w++ = 't'; }
        else { *w++ = *s; }
        s++;
    }
    *w++ = '"'; *w = 0;
    return buf;
}
