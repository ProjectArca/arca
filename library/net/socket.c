#include "../runtime/arca_runtime.h"

int32_t arca_net_socket(int32_t domain, int32_t type, int32_t protocol) {
    return (int32_t)socket(domain, type, protocol);
}

int32_t arca_net_setsockopt(int32_t fd, int32_t level, int32_t optname, void* optval, int32_t optlen) {
    return (int32_t)setsockopt(fd, level, optname, optval, (socklen_t)optlen);
}

int32_t arca_net_bind(int32_t fd, void* addr, int32_t addrlen) {
    return (int32_t)bind(fd, (struct sockaddr*)addr, (socklen_t)addrlen);
}

int32_t arca_net_listen(int32_t fd, int32_t backlog) {
    return (int32_t)listen(fd, backlog);
}

int32_t arca_net_accept(int32_t fd, void* addr, void* addrlen) {
    return (int32_t)accept(fd, (struct sockaddr*)addr, (socklen_t*)addrlen);
}

int32_t arca_net_close(int32_t fd) {
    return (int32_t)close(fd);
}

int32_t arca_net_write(int32_t fd, const void* buf, int32_t count) {
    return (int32_t)write(fd, buf, (size_t)count);
}

int32_t arca_net_read(int32_t fd, void* buf, int32_t count) {
    return (int32_t)read(fd, buf, (size_t)count);
}

int16_t arca_net_htons(int16_t hostshort) {
    return (int16_t)htons((uint16_t)hostshort);
}

void* arca_net_signal(int32_t sig, int32_t handler) {
    signal(sig, (void*)(intptr_t)handler);
    return NULL;
}
