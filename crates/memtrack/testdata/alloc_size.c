#include <stdlib.h>
#include <unistd.h>

int main() {
    sleep(1);
    void* ptr1 = malloc(1024);
    void* ptr2 = malloc(2048);
    void* ptr3 = malloc(512);
    void* ptr4 = malloc(4096);

    free(ptr1);
    free(ptr2);
    free(ptr3);
    free(ptr4);

    return 0;
}
