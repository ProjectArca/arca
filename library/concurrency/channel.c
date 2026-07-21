#include "../runtime/arca_runtime.h"

typedef struct {
    pthread_mutex_t lock;
    pthread_cond_t cond;
    void** buffer;
    size_t capacity;
    size_t head;
    size_t tail;
    size_t count;
} ArcaChannel;

void* arca_channel_create(size_t capacity) {
    ArcaChannel* chan = (ArcaChannel*)malloc(sizeof(ArcaChannel));
    pthread_mutex_init(&chan->lock, NULL);
    pthread_cond_init(&chan->cond, NULL);
    chan->capacity = capacity > 0 ? capacity : 16;
    chan->buffer = (void**)malloc(sizeof(void*) * chan->capacity);
    chan->head = 0;
    chan->tail = 0;
    chan->count = 0;
    return chan;
}
