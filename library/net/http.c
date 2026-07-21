#include "../runtime/arca_runtime.h"

int32_t arca_http_respond_json(int32_t fd, const char* json_body) {
    char header[512];
    int body_len = json_body ? (int)strlen(json_body) : 0;
    int header_len = snprintf(header, sizeof(header),
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s",
        body_len, json_body ? json_body : "");
    if (write(fd, header, header_len) <= 0) return -1;
    return 0;
}
