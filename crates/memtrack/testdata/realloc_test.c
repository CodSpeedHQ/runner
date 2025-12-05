#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* p = malloc(100);
    p = realloc(p, 200);
    free(p);
    return 0;
}
