#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p = malloc(512);
    free(p);
    return 0;
}
