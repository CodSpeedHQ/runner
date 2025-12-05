#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p = aligned_alloc(64, 512);
    free(p);
    return 0;
}
