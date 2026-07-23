# Arca Project Makefile
# Commands for building compiler, runtime library, compiling code, running examples, and benchmarking.

.PHONY: all build runtime compile run bench clean help install update test test-all

CC ?= cc
CFLAGS ?= -O3 -flto -march=native
CARGO ?= cargo
ARCA_CLI ?= ./target/release/arca-cli
RUNTIME_LIB ?= build/libarca_runtime.a
INSTALL_DIR ?= $(HOME)/.arca
INSTALL_BIN ?= $(INSTALL_DIR)/bin

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

# Install to ~/.arca/bin (local install, force replaces)
install: build
	@echo "[arca] Installing Arca to $(INSTALL_BIN)..."
	@mkdir -p $(INSTALL_BIN)
	@cp target/release/arca-cli $(INSTALL_BIN)/arca
	@chmod +x $(INSTALL_BIN)/arca
	@echo "[arca] Installed to $(INSTALL_BIN)/arca"
	@if ! grep -q '$(INSTALL_BIN)' ~/.zshrc 2>/dev/null; then \
		echo 'export PATH="$(INSTALL_BIN):$$PATH"' >> ~/.zshrc; \
		echo "[arca] Added $(INSTALL_BIN) to PATH in ~/.zshrc"; \
	fi
	@if ! grep -q '$(INSTALL_BIN)' ~/.bashrc 2>/dev/null; then \
		echo 'export PATH="$(INSTALL_BIN):$$PATH"' >> ~/.bashrc; \
		echo "[arca] Added $(INSTALL_BIN) to PATH in ~/.bashrc"; \
	fi
	@echo ""
	@echo "[arca] ✅ Installation complete!"
	@echo "    Run: source ~/.zshrc && arca --version"
	@$(INSTALL_BIN)/arca --version

# Update: build latest and reinstall (force replace)
update: build install
	@echo "[arca] ✅ Update complete!"

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

# Run Arca test suite
test: build
	@echo "[arca] Running test suite..."
	$(ARCA_CLI) test tests

# Run all tests including runtime tests
test-all: build
	@echo "[arca] Running full test suite..."
	$(ARCA_CLI) test tests

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
	@echo "  make install             Install arca to ~/.arca/bin (force replaces)"
	@echo "  make update              Build latest and reinstall"
	@echo "  make compile FILE=<path> Compile an Arca source file to native executable"
	@echo "  make run FILE=<path>    Compile and execute an Arca source file"
	@echo "  make bench              Run full benchmark suite (Arca vs Rust vs Go vs Bun)"
	@echo "  make test               Run Arca test suite"
	@echo "  make clean              Clean build directory and temporary binaries"
