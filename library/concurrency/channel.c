#include "../runtime/arca_runtime.h"

typedef struct {
    pthread_mutex_t lock;
    pthread_cond_t cond_read;
    pthread_cond_t cond_write;
    int64_t* buffer;
    size_t capacity;
    size_t head;
    size_t tail;
    size_t count;
} ArcaChannel;

void* arca_channel_create(size_t capacity) {
    ArcaChannel* chan = (ArcaChannel*)malloc(sizeof(ArcaChannel));
    pthread_mutex_init(&chan->lock, NULL);
    pthread_cond_init(&chan->cond_read, NULL);
    pthread_cond_init(&chan->cond_write, NULL);
    chan->capacity = capacity > 0 ? capacity : 16;
    chan->buffer = (int64_t*)malloc(sizeof(int64_t) * chan->capacity);
    chan->head = 0;
    chan->tail = 0;
    chan->count = 0;
    return chan;
}

void arca_channel_send(void* channel, int64_t val) {
    if (!channel) return;
    ArcaChannel* chan = (ArcaChannel*)channel;
    pthread_mutex_lock(&chan->lock);
    while (chan->count == chan->capacity) {
        pthread_cond_wait(&chan->cond_write, &chan->lock);
    }
    chan->buffer[chan->tail] = val;
    chan->tail = (chan->tail + 1) % chan->capacity;
    chan->count++;
    pthread_cond_signal(&chan->cond_read);
    pthread_mutex_unlock(&chan->lock);
}

int64_t arca_channel_recv(void* channel) {
    if (!channel) return 0;
    ArcaChannel* chan = (ArcaChannel*)channel;
    pthread_mutex_lock(&chan->lock);
    while (chan->count == 0) {
        pthread_cond_wait(&chan->cond_read, &chan->lock);
    }
    int64_t val = chan->buffer[chan->head];
    chan->head = (chan->head + 1) % chan->capacity;
    chan->count--;
    pthread_cond_signal(&chan->cond_write);
    pthread_mutex_unlock(&chan->lock);
    return val;
}
