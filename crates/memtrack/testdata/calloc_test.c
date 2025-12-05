#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p = calloc(10, 100);
    free(p);
    return 0;
}
