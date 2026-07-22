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

// std/io: stdout/stderr
void arca_stdout_write(const char* s) {
    if (s) fputs(s, stdout);
}

void arca_stderr_write(const char* s) {
    if (s) fputs(s, stderr);
}

// std/fs: extended operations
int64_t arca_fs_read(int64_t handle, void* buf, int64_t count) {
    if (!handle) return -1;
    return (int64_t)fread(buf, 1, (size_t)count, (FILE*)handle);
}

int64_t arca_fs_write(int64_t handle, const char* data, int64_t count) {
    if (!handle) return -1;
    return (int64_t)fwrite(data, 1, (size_t)count, (FILE*)handle);
}

int32_t arca_fs_rename(const char* old, const char* new_) {
    if (!old || !new_) return -1;
    return rename(old, new_) == 0 ? 0 : -1;
}

int32_t arca_fs_copy(const char* src, const char* dst) {
    if (!src || !dst) return -1;
    FILE* in = fopen(src, "rb");
    if (!in) return -1;
    FILE* out = fopen(dst, "wb");
    if (!out) { fclose(in); return -1; }
    char buf[8192];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), in)) > 0) {
        fwrite(buf, 1, n, out);
    }
    fclose(in); fclose(out);
    return 0;
}

int64_t arca_fs_metadata(const char* path) {
    if (!path) return 0;
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    // Encode size(48 bits) + mode(16 bits) into int64_t
    return (st.st_size & 0xFFFFFFFFFFFFLL) | ((int64_t)(st.st_mode & 0xFFFF) << 48);
}

// std/path: normalize (resolve . and ..)
const char* arca_path_normalize(const char* path) {
    if (!path || !*path) return "/";
    static char buf[4096];
    char temp[4096];
    strncpy(temp, path, sizeof(temp) - 1);
    temp[sizeof(temp) - 1] = 0;
    char* parts[256];
    int nparts = 0;
    char* tok = strtok(temp, "/");
    while (tok && nparts < 256) {
        if (strcmp(tok, ".") == 0) { tok = strtok(NULL, "/"); continue; }
        if (strcmp(tok, "..") == 0) { if (nparts > 0) nparts--; }
        else { parts[nparts++] = tok; }
        tok = strtok(NULL, "/");
    }
    buf[0] = '/'; buf[1] = 0;
    for (int i = 0; i < nparts; i++) {
        strcat(buf, parts[i]);
        if (i < nparts - 1) strcat(buf, "/");
    }
    return buf;
}

// Phase 3: Collections — Vec (dynamic array)
typedef struct { int64_t* data; int64_t len; int64_t cap; } ArcaVec;

int64_t arca_vec_new(void) {
    ArcaVec* v = (ArcaVec*)malloc(sizeof(ArcaVec));
    v->data = NULL; v->len = 0; v->cap = 0;
    return (int64_t)v;
}

int64_t arca_vec_len(int64_t handle) {
    if (!handle) return 0;
    return ((ArcaVec*)handle)->len;
}

void arca_vec_push(int64_t handle, int64_t val) {
    if (!handle) return;
    ArcaVec* v = (ArcaVec*)handle;
    if (v->len >= v->cap) {
        v->cap = v->cap ? v->cap * 2 : 8;
        v->data = (int64_t*)realloc(v->data, v->cap * sizeof(int64_t));
    }
    v->data[v->len++] = val;
}

int64_t arca_vec_get(int64_t handle, int64_t index) {
    if (!handle) return 0;
    ArcaVec* v = (ArcaVec*)handle;
    if (index < 0 || index >= v->len) return 0;
    return v->data[index];
}

void arca_vec_free(int64_t handle) {
    if (!handle) return;
    ArcaVec* v = (ArcaVec*)handle;
    free(v->data);
    free(v);
}

// Phase 2 implementations
const char* arca_str_split(const char* s, const char* delim, int index) {
    if (!s || !delim) return "";
    static char buf[4096];
    char temp[4096];
    strncpy(temp, s, sizeof(temp) - 1);
    temp[sizeof(temp) - 1] = 0;
    char* tok = strtok(temp, delim);
    int idx = 0;
    while (tok) {
        if (idx == index) {
            strncpy(buf, tok, sizeof(buf) - 1);
            buf[sizeof(buf) - 1] = 0;
            return buf;
        }
        tok = strtok(NULL, delim);
        idx++;
    }
    return "";
}

const char* arca_str_replace(const char* s, const char* from, const char* to) {
    if (!s || !from || !to) return s ? s : "";
    static char result[8192];
    result[0] = 0;
    const char* p = s;
    size_t from_len = strlen(from);
    if (from_len == 0) return s;
    while (*p) {
        if (strncmp(p, from, from_len) == 0) {
            strncat(result, to, sizeof(result) - strlen(result) - 1);
            p += from_len;
        } else {
            size_t rlen = strlen(result);
            if (rlen < sizeof(result) - 1) {
                result[rlen] = *p;
                result[rlen + 1] = 0;
            }
            p++;
        }
    }
    return result;
}

const char* arca_str_format(const char* fmt, const char* arg) {
    if (!fmt) return "";
    static char buf[4096];
    snprintf(buf, sizeof(buf), fmt, arg ? arg : "");
    return buf;
}

int64_t arca_duration_ms(int64_t ms) { return ms; }
int64_t arca_timer_start(void) { return arca_time_ms(); }
int64_t arca_timer_stop(int64_t timer) { return arca_time_ms() - timer; }

const char* arca_env_args(int index) {
    (void)index;
    return "";
}

#include <sys/stat.h>
#include <dirent.h>
int32_t arca_fs_mkdir(const char* path) {
    if (!path) return -1;
    return mkdir(path, 0755);
}

int32_t arca_fs_rmdir(const char* path) {
    if (!path) return -1;
    return rmdir(path);
}

const char* arca_fs_read_dir(const char* path) {
    if (!path) return "";
    DIR* d = opendir(path);
    if (!d) return "";
    static char buf[8192];
    buf[0] = 0;
    struct dirent* dir;
    while ((dir = readdir(d)) != NULL) {
        if (buf[0] != 0) strcat(buf, ",");
        strncat(buf, dir->d_name, sizeof(buf) - strlen(buf) - 1);
    }
    closedir(d);
    return buf;
}

int64_t arca_process_command(const char* cmd) {
    if (!cmd) return -1;
    return (int64_t)system(cmd);
}

int64_t arca_process_spawn(const char* cmd) {
    return arca_process_command(cmd);
}

int32_t arca_process_wait(int64_t pid) {
    (void)pid;
    return 0;
}

int32_t arca_tcp_connect(const char* host, int32_t port) {
    (void)host; (void)port;
    return 0;
}

int32_t arca_udp_bind(int32_t port) {
    (void)port;
    return 0;
}

int32_t arca_udp_send_to(int32_t fd, const char* msg, const char* host, int32_t port) {
    (void)fd; (void)msg; (void)host; (void)port;
    return 0;
}

const char* arca_udp_recv_from(int32_t fd) {
    (void)fd;
    return "";
}

int32_t arca_http_router_add(const char* method, const char* path) {
    (void)method; (void)path;
    return 0;
}

const char* arca_http_header_get(const char* headers, const char* key) {
    (void)headers; (void)key;
    return "";
}

const char* arca_http_cookie_get(const char* cookies, const char* key) {
    (void)cookies; (void)key;
    return "";
}

int32_t arca_ws_upgrade(int32_t fd) {
    (void)fd;
    return 0;
}

int32_t arca_sse_send(int32_t fd, const char* data) {
    (void)fd; (void)data;
    return 0;
}

const char* arca_json_parse(const char* json_str, const char* key) {
    if (!json_str || !key) return "";
    static char buf[4096];
    char search_pattern[256];
    snprintf(search_pattern, sizeof(search_pattern), "\"%s\"", key);
    const char* pos = strstr(json_str, search_pattern);
    if (!pos) return "";
    const char* val = strchr(pos, ':');
    if (!val) return "";
    val++;
    while (*val == ' ' || *val == '\t') val++;
    if (*val == '"') {
        val++;
        const char* end = strchr(val, '"');
        if (end) {
            size_t len = end - val;
            if (len >= sizeof(buf)) len = sizeof(buf) - 1;
            strncpy(buf, val, len);
            buf[len] = 0;
            return buf;
        }
    }
    size_t len = 0;
    while (val[len] && val[len] != ',' && val[len] != '}' && val[len] != ']') len++;
    if (len >= sizeof(buf)) len = sizeof(buf) - 1;
    strncpy(buf, val, len);
    buf[len] = 0;
    return buf;
}

