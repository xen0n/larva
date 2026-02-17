/*
 * Simple test program using musl libc
 * Compiled with debug symbols for debugging LARVa
 */

#include <stdio.h>

int main(int argc, const char *argv[])
{
    const char *name = "world";
    if (argc >= 2)
        name = argv[1];
    printf("Hello, %s!\n", name);
    return 0;
}
