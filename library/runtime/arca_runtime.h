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

// Print helpers
void arca_print_int(int64_t v);
void arca_print_string(const char* s);
void arca_println_int(int64_t v);
void arca_println_string(const char* s);

// Monotonic Time helpers (64-bit nanosecond and millisecond resolution)
int64_t arca_time_ns(void);
int64_t arca_time_ms(void);

// OS Socket & Network helpers
int32_t arca_socket(int32_t domain, int32_t type, int32_t protocol);
int32_t arca_setsockopt(int32_t fd, int32_t level, int32_t optname, void* optval, int32_t optlen);
int32_t arca_bind(int32_t fd, void* addr, int32_t addrlen);
int32_t arca_listen(int32_t fd, int32_t backlog);
int32_t arca_accept(int32_t fd, void* addr, void* addrlen);
int32_t arca_close(int32_t fd);
int32_t arca_write(int32_t fd, const void* buf, int32_t count);
int32_t arca_read(int32_t fd, void* buf, int32_t count);
int16_t arca_htons(int16_t hostshort);
void* arca_signal(int32_t sig, int32_t handler);

// Concurrency helper (temporary bootstrap worker thread wrapper)
void arca_handle_client_async(int32_t fd);

#ifdef __cplusplus
}
#endif

#endif // ARCA_RUNTIME_H
