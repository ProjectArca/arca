#include "../runtime/arca_runtime.h"
#include <errno.h>
#include <fcntl.h>

#ifndef SOCK_NONBLOCK
#define SOCK_NONBLOCK 0
#endif

#define NUM_WORKERS 128
#define QUEUE_SIZE 65536

static int g_queue[QUEUE_SIZE];
static int g_q_head = 0;
static int g_q_tail = 0;
static int g_q_count = 0;
static pthread_mutex_t g_q_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t g_q_cond = PTHREAD_COND_INITIALIZER;
static ArcaHttpHandlerFn g_handler = NULL;

static void parse_http_request(const char* buf, ArcaHttpRequest* req) {
    req->method = "GET";
    req->path = "/";

    if (strncmp(buf, "GET", 3) == 0) req->method = "GET";
    else if (strncmp(buf, "POST", 4) == 0) req->method = "POST";
    else if (strncmp(buf, "PUT", 3) == 0) req->method = "PUT";
    else if (strncmp(buf, "DELETE", 6) == 0) req->method = "DELETE";

    const char* p1 = strchr(buf, ' ');
    if (p1) {
        p1++;
        const char* p2 = strchr(p1, ' ');
        if (p2) {
            static char path_buf[256];
            size_t len = (size_t)(p2 - p1);
            if (len >= sizeof(path_buf)) len = sizeof(path_buf) - 1;
            memcpy(path_buf, p1, len);
            path_buf[len] = '\0';
            req->path = path_buf;
        }
    }
}

static void process_client(int client_fd) {
    char buf[4096];
    ssize_t n = read(client_fd, buf, sizeof(buf) - 1);
    if (n > 0) {
        buf[n] = '\0';
        ArcaHttpRequest req;
        parse_http_request(buf, &req);

        ArcaHttpResponse res;
        if (g_handler) {
            res = g_handler(req);
        } else {
            res.status = 200;
            res.content_type = "application/json";
            res.body = "{\"message\": \"hello\"}";
        }

        if (!res.content_type) res.content_type = "application/json";
        if (!res.body) res.body = "{\"message\": \"hello\"}";
        if (res.status == 0) res.status = 200;

        char header[8192];
        int body_len = (int)strlen(res.body);
        int header_len = snprintf(header, sizeof(header),
            "HTTP/1.1 %d OK\r\nContent-Type: %s\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s",
            res.status, res.content_type, body_len, res.body);

        if (header_len > 0) {
            write(client_fd, header, header_len);
        }
    }
    close(client_fd);
}

static void* arca_http_worker(void* arg) {
    (void)arg;
    while (1) {
        int client_fd = -1;

        pthread_mutex_lock(&g_q_lock);
        while (g_q_count == 0) {
            pthread_cond_wait(&g_q_cond, &g_q_lock);
        }
        client_fd = g_queue[g_q_head];
        g_q_head = (g_q_head + 1) % QUEUE_SIZE;
        g_q_count--;
        pthread_mutex_unlock(&g_q_lock);

        if (client_fd >= 0) {
            process_client(client_fd);
        }
    }
    return NULL;
}

int32_t arca_std_http_serve_handler(int32_t port, ArcaHttpHandlerFn handler) {
    g_handler = handler;
    return arca_std_http_serve(port);
}

int32_t arca_std_http_serve(int32_t port) {
    signal(SIGPIPE, SIG_IGN);

    pthread_t workers[NUM_WORKERS];
    for (int i = 0; i < NUM_WORKERS; i++) {
        pthread_create(&workers[i], NULL, arca_http_worker, NULL);
        pthread_detach(workers[i]);
    }

    int sock = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0);
    if (sock < 0) {
        // Fallback: macOS doesn't support SOCK_NONBLOCK
        sock = socket(AF_INET, SOCK_STREAM, 0);
        if (sock >= 0) {
            int flags = fcntl(sock, F_GETFL, 0);
            fcntl(sock, F_SETFL, flags | O_NONBLOCK);
        }
    }
    if (sock < 0) return -1;

    int optval = 1;
    setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &optval, sizeof(optval));
#ifdef SO_REUSEPORT
    setsockopt(sock, SOL_SOCKET, SO_REUSEPORT, &optval, sizeof(optval));
#endif

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    addr.sin_addr.s_addr = INADDR_ANY;

    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(sock);
        return -1;
    }

    if (listen(sock, 16384) < 0) {
        close(sock);
        return -1;
    }

    printf("Arca HTTP server listening on port %d\n", port);
    fflush(stdout);

    while (1) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);
        int client_fd = accept(sock, (struct sockaddr*)&client_addr, &client_len);
        if (client_fd >= 0) {
            pthread_mutex_lock(&g_q_lock);
            if (g_q_count < QUEUE_SIZE) {
                g_queue[g_q_tail] = client_fd;
                g_q_tail = (g_q_tail + 1) % QUEUE_SIZE;
                g_q_count++;
                pthread_cond_signal(&g_q_cond);
                pthread_mutex_unlock(&g_q_lock);
            } else {
                pthread_mutex_unlock(&g_q_lock);
                close(client_fd);
            }
        } else if (errno != EAGAIN && errno != EWOULDBLOCK) {
            usleep(1000);
        }
    }
    close(sock);
    return 0;
}
