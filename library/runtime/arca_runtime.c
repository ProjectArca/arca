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

int32_t arca_socket(int32_t domain, int32_t type, int32_t protocol) {
    return (int32_t)socket(domain, type, protocol);
}

int32_t arca_setsockopt(int32_t fd, int32_t level, int32_t optname, void* optval, int32_t optlen) {
    return (int32_t)setsockopt(fd, level, optname, optval, (socklen_t)optlen);
}

int32_t arca_bind(int32_t fd, void* addr, int32_t addrlen) {
    return (int32_t)bind(fd, (struct sockaddr*)addr, (socklen_t)addrlen);
}

int32_t arca_listen(int32_t fd, int32_t backlog) {
    return (int32_t)listen(fd, backlog);
}

int32_t arca_accept(int32_t fd, void* addr, void* addrlen) {
    return (int32_t)accept(fd, (struct sockaddr*)addr, (socklen_t*)addrlen);
}

int32_t arca_close(int32_t fd) {
    return (int32_t)close(fd);
}

int32_t arca_write(int32_t fd, const void* buf, int32_t count) {
    return (int32_t)write(fd, buf, (size_t)count);
}

int32_t arca_read(int32_t fd, void* buf, int32_t count) {
    return (int32_t)read(fd, buf, (size_t)count);
}

int16_t arca_htons(int16_t hostshort) {
    return (int16_t)htons((uint16_t)hostshort);
}

void* arca_signal(int32_t sig, int32_t handler) {
    signal(sig, (void*)(intptr_t)handler);
    return NULL;
}

static void* arca_client_thread(void* arg) {
    int fd = (int)(intptr_t)arg;
    char buf[512];
    const char* res = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\nConnection: close\r\n\r\n{\"message\": \"hello\"}";
    int len = (int)strlen(res);
    if (read(fd, buf, sizeof(buf)) > 0) {
        write(fd, res, len);
    }
    close(fd);
    return NULL;
}

void arca_handle_client_async(int32_t fd) {
    pthread_t t;
    if (pthread_create(&t, NULL, arca_client_thread, (void*)(intptr_t)fd) == 0) {
        pthread_detach(t);
    } else {
        close(fd);
    }
}
