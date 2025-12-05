#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p1 = malloc(100);
    void* p2 = malloc(200);
    void* p3 = malloc(300);
    free(p1);
    free(p2);
    free(p3);
    return 0;
}
