#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
    sleep(1);
    // Parent allocates
    void* p_parent = malloc(256);

    pid_t child_pid = fork();

    if (child_pid == 0) {
        // Child allocates
        void* p_child = malloc(512);
        free(p_child);
        exit(0);
    } else {
        // Parent waits for child
        waitpid(child_pid, NULL, 0);
        free(p_parent);
    }

    return 0;
}
