#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p1 = malloc(1024);
    void* p2 = malloc(2048);
    free(p1);
    free(p2);
    return 0;
}
