#include "../runtime/arca_runtime.h"

typedef struct {
    uint64_t actor_id;
    void* mailbox;
} ArcaActor;

void* arca_actor_create(uint64_t id) {
    ArcaActor* actor = (ArcaActor*)malloc(sizeof(ArcaActor));
    actor->actor_id = id;
    actor->mailbox = NULL;
    return actor;
}
