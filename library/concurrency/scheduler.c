#include "../runtime/arca_runtime.h"

void arca_scheduler_init(int threads) {
    (void)threads;
}

void arca_scheduler_spawn(void (*func)(void*), void* arg) {
    pthread_t t;
    if (pthread_create(&t, NULL, (void*(*)(void*))func, arg) == 0) {
        pthread_detach(t);
    }
}
