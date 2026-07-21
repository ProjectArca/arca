# Arca Project Makefile
# Commands for building compiler, runtime library, compiling code, running examples, and benchmarking.

.PHONY: all build runtime compile run bench clean help

CC ?= cc
CFLAGS ?= -O3 -flto -march=native
CARGO ?= cargo
ARCA_CLI ?= ./target/release/arca-cli
RUNTIME_LIB ?= build/libarca_runtime.a

all: build runtime

# Build the Arca compiler CLI binary (release mode)
build:
	@echo "[arca] Building Arca Compiler CLI..."
	$(CARGO) build --release

# Build the modular native C runtime static library (libarca_runtime.a)
runtime:
	@echo "[arca] Building Arca Runtime Library ($(RUNTIME_LIB))..."
	@mkdir -p build
	$(CC) $(CFLAGS) -c library/runtime/arca_runtime.c library/core/*.c library/net/*.c library/concurrency/*.c library/fs/*.c library/alloc/*.c -I library/runtime
	ar rcs $(RUNTIME_LIB) *.o
	@rm -f *.o

# Compile an Arca source file to native executable
# Usage: make compile FILE=benchmarks/web_api/server.arca
compile: build runtime
	@if [ -z "$(FILE)" ]; then echo "Error: Please specify FILE, e.g., make compile FILE=examples/http_server.arca"; exit 1; fi
	@echo "[arca] Compiling $(FILE)..."
	$(ARCA_CLI) build $(FILE)
	$(CC) $(CFLAGS) -o /tmp/arca_exec build/output.c $(RUNTIME_LIB) -lpthread
	@echo "[arca] Native binary compiled successfully to /tmp/arca_exec"

# Compile and run an Arca source file
# Usage: make run FILE=examples/http_server.arca
run: compile
	@echo "[arca] Running /tmp/arca_exec..."
	@/tmp/arca_exec

# Run full benchmark suite across Arca, Rust, Go, and Bun
bench: build runtime
	@echo "[arca] Executing benchmark suite..."
	./benchmarks/run.sh

# Clean build artifacts
clean:
	@echo "[arca] Cleaning build directory..."
	$(CARGO) clean
	rm -rf build /tmp/arca_*

# Show help menu
help:
	@echo "Arca Makefile Targets:"
	@echo "  make build               Build Arca compiler CLI (release mode)"
	@echo "  make runtime             Build native C runtime static library (libarca_runtime.a)"
	@echo "  make compile FILE=<path> Compile an Arca source file to native executable"
	@echo "  make run FILE=<path>     Compile and execute an Arca source file"
	@echo "  make bench               Run full benchmark suite (Arca vs Rust vs Go vs Bun)"
	@echo "  make clean               Clean build directory and temporary binaries"
