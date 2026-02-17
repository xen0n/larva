# Makefile for building test programs with custom musl
# This creates debuggable binaries for LARVa testing

MUSL_ROOT ?= $(CURDIR)/musl-riscv64
CC = $(MUSL_ROOT)/bin/riscv64-linux-musl-gcc

CFLAGS = -g -O0 -fno-omit-frame-pointer -static
LDFLAGS = -static

TARGETS = hello-argv.elf

.PHONY: all clean check-musl

all: check-musl $(TARGETS)

check-musl:
	@if [ ! -f "$(CC)" ]; then \
		echo "Error: musl cross-compiler not found at $(CC)"; \
		echo "Run ./build-musl.sh first to build musl"; \
		exit 1; \
	fi

hello-argv.elf: hello-argv.c
	$(CC) $(CFLAGS) -o $@ $<
	@echo "Built: $@"
	@echo "File info:"
	@file $@
	@echo "Stripped size: $$(stat -c%s $@ 2>/dev/null || stat -f%z $@) bytes"

clean:
	rm -f $(TARGETS)

# Build with verbose output for debugging
verbose: check-musl
	$(CC) $(CFLAGS) -v -o hello-argv.elf hello-argv.c 2>&1

# Build and test with LARVa
test: hello-argv.elf
	cd .. && cargo run --bin larva-run -- ./test/hello-argv.elf

# Disassemble for debugging
disasm: hello-argv.elf
	riscv64-linux-gnu-objdump -d $< 2>/dev/null || \
	riscv64-unknown-linux-gnu-objdump -d $< 2>/dev/null || \
	echo "No objdump found"
