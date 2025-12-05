#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void** ptrs = malloc(sizeof(void*) * 100);

    // Allocate 100 times
    for (int i = 0; i < 100; i++) {
        ptrs[i] = malloc(64);
    }

    // Free them all
    for (int i = 0; i < 100; i++) {
        free(ptrs[i]);
    }

    free(ptrs);
    return 0;
}
