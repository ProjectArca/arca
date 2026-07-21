#include "../runtime/arca_runtime.h"

static void* arca_tcp_client_thread(void* arg) {
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

void arca_net_handle_async(int32_t fd) {
    pthread_t t;
    if (pthread_create(&t, NULL, arca_tcp_client_thread, (void*)(intptr_t)fd) == 0) {
        pthread_detach(t);
    } else {
        close(fd);
    }
}
