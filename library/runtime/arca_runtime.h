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

// Concurrency Scheduler Helpers
void arca_scheduler_init(int threads);
void arca_scheduler_spawn(void (*func)(void*), void* arg);
void* arca_channel_create(size_t capacity);
void arca_channel_send(void* channel, int64_t val);
int64_t arca_channel_recv(void* channel);

// Memory & Allocator Helpers
void* arca_mem_alloc(size_t size);
void arca_mem_free(void* ptr);

#ifdef __cplusplus
}
#endif

#endif // ARCA_RUNTIME_H
