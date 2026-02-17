# Makefile for minimal RISC-V test program
# This creates a static binary with no libc dependency

CROSS_PREFIX ?= riscv64-linux-gnu-
AS = $(CROSS_PREFIX)as
LD = $(CROSS_PREFIX)ld

TARGET = hello.elf
OBJS = hello.o

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(OBJS)
	$(LD) -o $@ $^

%.o: %.s
	$(AS) -o $@ $<

clean:
	rm -f $(OBJS) $(TARGET)

# Alternative: use GCC to handle linking
hello-gcc.elf: hello.s
	$(CROSS_PREFIX)gcc -nostdlib -static -o $@ $<

.PHONY: test
test: $(TARGET)
	@echo "Run with: cargo run --bin larva-run -- ./$(TARGET)"
