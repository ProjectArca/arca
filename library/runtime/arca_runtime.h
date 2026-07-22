#ifndef ARCA_RUNTIME_H
#define ARCA_RUNTIME_H

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
#include <signal.h>
#include <pthread.h>
#include <math.h>
#include <sys/socket.h>
#include <netinet/in.h>

#ifdef __cplusplus
extern "C" {
#endif

// Core Print & String Helpers
void arca_print_int(int64_t v);
void arca_print_string(const char* s);
void arca_println_int(int64_t v);
void arca_println_string(const char* s);

// Monotonic Time Helpers (64-bit nanosecond and millisecond resolution)
int64_t arca_time_ns(void);
int64_t arca_time_ms(void);

// Net & Socket Helpers
int32_t arca_net_socket(int32_t domain, int32_t type, int32_t protocol);
int32_t arca_net_setsockopt(int32_t fd, int32_t level, int32_t optname, void* optval, int32_t optlen);
int32_t arca_net_bind(int32_t fd, void* addr, int32_t addrlen);
int32_t arca_net_listen(int32_t fd, int32_t backlog);
int32_t arca_net_accept(int32_t fd, void* addr, void* addrlen);
int32_t arca_net_close(int32_t fd);
int32_t arca_net_write(int32_t fd, const void* buf, int32_t count);
int32_t arca_net_read(int32_t fd, void* buf, int32_t count);
int16_t arca_net_htons(int16_t hostshort);
void* arca_net_signal(int32_t sig, int32_t handler);
void arca_net_handle_async(int32_t fd);

// High-level Standard Library HTTP & TCP Abstractions
typedef struct {
    const char* method;
    const char* path;
} ArcaHttpRequest;

typedef struct {
    int32_t status;
    const char* content_type;
    const char* body;
} ArcaHttpResponse;

typedef ArcaHttpResponse (*ArcaHttpHandlerFn)(ArcaHttpRequest);

int32_t arca_std_http_serve(int32_t port);
int32_t arca_std_http_serve_handler(int32_t port, ArcaHttpHandlerFn handler);
int32_t arca_http_add_route(const char* method, const char* pattern, ArcaHttpHandlerFn handler);
int32_t arca_http_set_default_handler(ArcaHttpHandlerFn handler);

// String helpers for CRUD
int32_t arca_strcmp(const char* a, const char* b);
int32_t arca_starts_with(const char* s, const char* prefix);
const char* arca_strcat(const char* a, const char* b);
int32_t arca_parse_int(const char* s);
const char* arca_int_to_str(int32_t n);
int32_t arca_str_rfind(const char* s, char c);
const char* arca_str_slice(const char* s, int32_t start);

// Extended string helpers (std/string)
const char* arca_str_trim(const char* s);
int32_t arca_str_contains(const char* s, const char* sub);
int32_t arca_ends_with(const char* s, const char* suffix);

// std/time
void arca_sleep_ms(int64_t ms);

// std/env
int64_t arca_env_get(const char* name);
int64_t arca_env_set(const char* name, const char* value);
int64_t arca_current_dir(void);

// std/io: stdin read line
const char* arca_stdin_read_line(void);
void arca_stdout_write(const char* s);
void arca_stderr_write(const char* s);

// std/fs
int64_t arca_fs_open(const char* path, const char* mode);
int32_t arca_fs_close(int64_t handle);
int32_t arca_fs_exists(const char* path);
int32_t arca_fs_remove(const char* path);
int64_t arca_fs_read(int64_t handle, void* buf, int64_t count);
int64_t arca_fs_write(int64_t handle, const char* data, int64_t count);
int32_t arca_fs_rename(const char* old, const char* new_);
int32_t arca_fs_copy(const char* src, const char* dst);
int64_t arca_fs_metadata(const char* path);

// std/path
const char* arca_path_extension(const char* path);
const char* arca_path_filename(const char* path);
const char* arca_path_parent(const char* path);
const char* arca_path_join(const char* a, const char* b);
const char* arca_path_normalize(const char* path);

// std/process
void arca_exit(int64_t code);

// std/json
const char* arca_json_stringify(const char* s);

// Concurrency Scheduler Helpers
void arca_scheduler_init(int threads);
void arca_scheduler_spawn(void (*func)(void*), void* arg);
void* arca_channel_create(size_t capacity);
void arca_channel_send(void* channel, int64_t val);
int64_t arca_channel_recv(void* channel);

// Error Handling Helpers
void __arca_throw(int64_t err);
int64_t __arca_get_last_error(void);
void __arca_clear_last_error(void);

// Result & Option Helpers
int64_t arca_result_ok(int64_t val);
int64_t arca_result_err(int64_t err);
int32_t arca_result_is_ok(int64_t res);
int64_t arca_result_unwrap(int64_t res);
int64_t arca_option_some(int64_t val);
int32_t arca_option_is_some(int64_t opt);

// Memory & Allocator Helpers
void* arca_mem_alloc(size_t size);
void arca_mem_free(void* ptr);

#ifdef __cplusplus
}
#endif

#endif // ARCA_RUNTIME_H
