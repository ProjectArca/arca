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
#include <sys/stat.h>
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
int64_t arca_str_len(const char* s);
int32_t arca_ends_with(const char* s, const char* suffix);
const char* arca_str_split(const char* s, const char* delim, int index);
const char* arca_str_replace(const char* s, const char* from, const char* to);
const char* arca_str_format(const char* fmt, const char* arg);

// Patch 1: std/string method implementations
int64_t __arca_str_is_empty(const char* s);
const char* __arca_str_at(const char* s, int64_t i);
const char* __arca_str_lower(const char* s);
const char* __arca_str_upper(const char* s);
const char* __arca_str_repeat(const char* s, int64_t n);
const char* __arca_str_lines(const char* s);
int32_t __arca_str_find(const char* s, const char* sub);
int32_t __arca_str_count(const char* s, const char* sub);
const char* __arca_hostname(void);
const char* __arca_username(void);

// Roadmap 3: Assertion API
void __arca_assert_eq(int64_t actual, int64_t expected);
void __arca_assert_throw(int64_t fn_ptr);
void __arca_match_snapshot(int64_t actual);

// std/time
void arca_sleep_ms(int64_t ms);
int64_t arca_duration_ms(int64_t ms);
int64_t arca_timer_start(void);
int64_t arca_timer_stop(int64_t timer);

// std/env
int64_t arca_env_get(const char* name);
int64_t arca_env_set(const char* name, const char* value);
int64_t arca_current_dir(void);
const char* arca_env_args(int index);

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
int32_t arca_fs_mkdir(const char* path);
int32_t arca_fs_rmdir(const char* path);
const char* arca_fs_read_dir(const char* path);

// std/fs high-level wrappers
const char* file_read(const char* path);
int32_t file_write(const char* path, const char* data);
int32_t file_append(const char* path, const char* data);
int32_t file_exists(const char* path);
int32_t file_copy(const char* src, const char* dst);
int32_t file_rename(const char* old, const char* new_);
int32_t file_remove(const char* path);
int32_t file_mkdir(const char* path);

// std/encoding
const char* hex_encode(const char* data);
const char* urlencode(const char* data);
const char* urldecode(const char* data);

// std/net high-level wrappers
int32_t tcp_listen(int32_t port);
int32_t tcp_accept(int32_t fd);
const char* tcp_recv(int32_t fd);

// std/path
const char* arca_path_extension(const char* path);
const char* arca_path_filename(const char* path);
const char* arca_path_parent(const char* path);
const char* arca_path_join(const char* a, const char* b);
const char* arca_path_normalize(const char* path);

// std/process
void arca_exit(int64_t code);
int64_t arca_process_command(const char* cmd);
int64_t arca_process_spawn(const char* cmd);
int32_t arca_process_wait(int64_t pid);

// std/net extended
int32_t arca_tcp_connect(const char* host, int32_t port);
int32_t arca_udp_bind(int32_t port);
int32_t arca_udp_send_to(int32_t fd, const char* msg, const char* host, int32_t port);
const char* arca_udp_recv_from(int32_t fd);

// std/http extra
int32_t arca_http_router_add(const char* method, const char* path);
const char* arca_http_header_get(const char* headers, const char* key);
const char* arca_http_cookie_get(const char* cookies, const char* key);
int32_t arca_ws_upgrade(int32_t fd);
int32_t arca_sse_send(int32_t fd, const char* data);

// std/json
const char* arca_json_stringify(const char* s);
const char* arca_json_parse(const char* json_str, const char* key);

// Phase 3: Collections (Vec, HashMap, HashSet, Queue, Deque, BinaryHeap, LinkedList)
int64_t arca_vec_new(void);
int64_t arca_vec_len(int64_t handle);
void arca_vec_push(int64_t handle, int64_t val);
int64_t arca_vec_get(int64_t handle, int64_t index);
int64_t arca_vec_pop(int64_t handle);
void arca_vec_free(int64_t handle);

int64_t arca_map_new(void);
void arca_map_insert(int64_t handle, const char* key, int64_t val);
int64_t arca_map_get(int64_t handle, const char* key);
int32_t arca_map_contains(int64_t handle, const char* key);
int64_t arca_map_len(int64_t handle);
void arca_map_free(int64_t handle);

int64_t arca_set_new(void);
void arca_set_add(int64_t handle, const char* key);
int32_t arca_set_contains(int64_t handle, const char* key);
int64_t arca_set_len(int64_t handle);
void arca_set_free(int64_t handle);

int64_t arca_queue_new(void);
void arca_queue_push(int64_t handle, int64_t val);
int64_t arca_queue_pop(int64_t handle);
int64_t arca_queue_len(int64_t handle);
void arca_queue_free(int64_t handle);

int64_t arca_deque_new(void);
void arca_deque_push_back(int64_t handle, int64_t val);
void arca_deque_push_front(int64_t handle, int64_t val);
int64_t arca_deque_pop_back(int64_t handle);
int64_t arca_deque_pop_front(int64_t handle);
int64_t arca_deque_len(int64_t handle);
void arca_deque_free(int64_t handle);

int64_t arca_heap_new(void);
void arca_heap_push(int64_t handle, int64_t val);
int64_t arca_heap_pop(int64_t handle);
int64_t arca_heap_len(int64_t handle);
void arca_heap_free(int64_t handle);

int64_t arca_list_new(void);
void arca_list_push_back(int64_t handle, int64_t val);
void arca_list_push_front(int64_t handle, int64_t val);
int64_t arca_list_pop_back(int64_t handle);
int64_t arca_list_pop_front(int64_t handle);
// Phase 4: Iterator
int64_t arca_iter_filter(int64_t handle, int64_t pred_fn);
int64_t arca_iter_map(int64_t handle, int64_t map_fn);
int64_t arca_iter_take(int64_t handle, int64_t count);
// Phase 6: AI Standard Library (Tensor, Dataset, Tokenizer, Embedding, Inference, SIMD, AI Providers, Vector DB, RAG)
int64_t arca_tensor_new(const char* shape);
int64_t arca_tensor_reshape(int64_t h, const char* new_shape);
int64_t arca_tensor_transpose(int64_t h);
int64_t arca_tensor_slice(int64_t h, int64_t start, int64_t end);
int64_t arca_tensor_broadcast(int64_t h, const char* shape);

int64_t arca_dataset_load(const char* path, const char* format);
int64_t arca_dataset_shuffle(int64_t h);
int64_t arca_dataset_batch(int64_t h, int64_t batch_size);
int64_t arca_dataset_split(int64_t h, int64_t ratio);

int64_t arca_tokenizer_load(const char* kind);
const char* arca_tokenizer_encode(int64_t h, const char* text);
const char* arca_tokenizer_decode(int64_t h, const char* tokens);

int64_t arca_embedding_cosine_similarity(int64_t a, int64_t b);
int64_t arca_embedding_normalize(int64_t h);
const char* arca_embedding_topk(int64_t h, int32_t k);

int64_t arca_inference_load(const char* model_path, const char* format);
int64_t arca_inference_predict(int64_t h, int64_t input);

int64_t arca_simd_dot_product(int64_t a, int64_t b);
int64_t arca_simd_matmul(int64_t a, int64_t b);

// AI Provider Integrations (OpenAI, Anthropic, Custom OpenAI-Compatible)
const char* arca_ai_chat_completion(const char* provider, const char* model, const char* prompt, const char* api_key, const char* base_url);
const char* arca_ai_embeddings(const char* provider, const char* model, const char* input, const char* api_key);
const char* arca_ai_request(const char* method, const char* url, const char* headers, const char* body, int32_t timeout);
const char* arca_chat(const char* model, const char* prompt, const char* system, double temperature, int32_t max_tokens);
const char* arca_embedding(const char* model, const char* input);
const char* arca_image(const char* prompt, const char* model, const char* size);
const char* arca_speech(const char* input, const char* voice);
const char* arca_transcribe(const char* audio_data);
const char* arca_claude(const char* prompt, int32_t max_tokens);
void arca_set_ai_config(const char* api_key, const char* base_url, int32_t timeout);

// Vector DB Integrations (Memory, PGVector, Qdrant, Chroma)
int64_t arca_vector_db_connect(const char* db_type, const char* conn_str);
int32_t arca_vector_db_insert(int64_t handle, const char* id, const char* vector_str, const char* metadata);
const char* arca_vector_db_search(int64_t handle, const char* query_vec_str, int32_t top_k);

// RAG Pipeline (Retrieval-Augmented Generation)
int64_t arca_rag_create(int64_t db_handle, const char* llm_provider, const char* model);
int32_t arca_rag_ingest_document(int64_t handle, const char* doc_text, int32_t chunk_size);
const char* arca_rag_query(int64_t handle, const char* query_text);
int64_t arca_future_create(void);
void arca_future_complete(int64_t fut, int64_t val);
int64_t arca_future_await(int64_t fut);
int64_t arca_task_spawn(void (*func)(void*), void* arg);
void arca_scheduler_spawn(void (*func)(void*), void* arg);
void* arca_channel_create(size_t capacity);
void arca_channel_send(void* channel, int64_t val);
int64_t arca_channel_recv(void* channel);

// Iterator Helpers
int64_t arca_iter_filter(int64_t h, int64_t pred_fn);
int64_t arca_iter_map(int64_t h, int64_t map_fn);
int64_t arca_iter_take(int64_t h, int64_t count);
int64_t arca_iter_skip(int64_t h, int64_t count);
int64_t arca_iter_collect(int64_t h);
int64_t arca_iter_reduce(int64_t h, int64_t fn, int64_t init);
int64_t arca_iter_enumerate(int64_t h);

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
