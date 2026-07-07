# Makefile for simed-tunnel
# Self-contained Tailscale tunnel for Sim AI

BINARY_NAME := simed-tunnel
VERSION := 1.0.0
BUILD_TIME := $(shell date -u '+%Y-%m-%d_%H:%M:%S')
GIT_COMMIT := $(shell git rev-parse HEAD 2>/dev/null || echo "unknown")

# Go build flags
GOFLAGS := -v
LDFLAGS := -ldflags="-s -w -X main.version=$(VERSION) -X main.buildTime=$(BUILD_TIME) -X main.gitCommit=$(GIT_COMMIT)"
LDFLAGS_DEBUG := -ldflags="-X main.version=$(VERSION) -X main.buildTime=$(BUILD_TIME) -X main.gitCommit=$(GIT_COMMIT)"

.PHONY: all build clean test run deps install uninstall

# Default target
all: build

# Initialize Go module and download dependencies
init:
	@echo "📦 Initializing Go module..."
	@go mod init github.com/yourusername/simed-tunnel 2>/dev/null || true
	@go mod tidy
	@echo "✓ Module initialized"

# Download dependencies
deps:
	@echo "📦 Downloading dependencies..."
	@go mod download
	@go mod tidy
	@echo "✓ Dependencies ready"

# Build the binary
build: deps
	@echo "🔨 Building $(BINARY_NAME)..."
	@CGO_ENABLED=0 go build $(GOFLAGS) $(LDFLAGS) -o $(BINARY_NAME) .
	@echo "✓ Built $(BINARY_NAME) successfully"
	@echo "📦 Binary size: $$(du -h $(BINARY_NAME) | cut -f1)"

# Build debug version (with symbols)
build-debug: deps
	@echo "🔨 Building debug version..."
	@CGO_ENABLED=0 go build $(GOFLAGS) $(LDFLAGS_DEBUG) -o $(BINARY_NAME)-debug .
	@echo "✓ Built debug binary"

# Build for all platforms
build-all: clean deps
	@echo "🔨 Building for multiple platforms..."
	@mkdir -p dist
	
	# Linux AMD64
	@echo "  → Building for linux/amd64..."
	@GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-linux-amd64 .
	
	# Linux ARM64
	@echo "  → Building for linux/arm64..."
	@GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-linux-arm64 .
	
	# macOS AMD64
	@echo "  → Building for darwin/amd64..."
	@GOOS=darwin GOARCH=amd64 CGO_ENABLED=0 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-darwin-amd64 .
	
	# macOS ARM64 (Apple Silicon)
	@echo "  → Building for darwin/arm64..."
	@GOOS=darwin GOARCH=arm64 CGO_ENABLED=0 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-darwin-arm64 .
	
	# Windows AMD64
	@echo "  → Building for windows/amd64..."
	@GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-windows-amd64.exe .
	
	@echo "✓ All builds completed"
	@ls -lh dist/

# Run the binary
run: build
	@echo "🚀 Running $(BINARY_NAME)..."
	@./$(BINARY_NAME)

# Run with verbose mode
run-verbose: build
	@echo "🚀 Running $(BINARY_NAME) in verbose mode..."
	@./$(BINARY_NAME) --verbose

# Run without starting simed (proxy mode)
run-proxy: build
	@echo "🚀 Running $(BINARY_NAME) in proxy mode..."
	@./$(BINARY_NAME) --no-simed

# Test the build
test: build
	@echo "🧪 Testing $(BINARY_NAME)..."
	@./$(BINARY_NAME) --version
	@echo "✓ Binary test passed"

# Install to system (requires sudo on some systems)
install: build
	@echo "📦 Installing $(BINARY_NAME)..."
	@install -d $(DESTDIR)/usr/local/bin
	@install -m 755 $(BINARY_NAME) $(DESTDIR)/usr/local/bin/
	@echo "✓ Installed to /usr/local/bin/$(BINARY_NAME)"

# Uninstall from system
uninstall:
	@echo "🗑️  Uninstalling $(BINARY_NAME)..."
	@rm -f $(DESTDIR)/usr/local/bin/$(BINARY_NAME)
	@echo "✓ Uninstalled"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning up..."
	@rm -f $(BINARY_NAME) $(BINARY_NAME)-debug
	@rm -rf dist/
	@echo "✓ Clean completed"

# Show binary info
info:
	@echo "📊 Binary Information:"
	@echo "  Name: $(BINARY_NAME)"
	@echo "  Version: $(VERSION)"
	@echo "  Build Time: $(BUILD_TIME)"
	@echo "  Git Commit: $(GIT_COMMIT)"
	@if [ -f $(BINARY_NAME) ]; then \
		echo "  Binary Size: $$(du -h $(BINARY_NAME) | cut -f1)"; \
		echo "  Binary Type: $$(file $(BINARY_NAME))"; \
	else \
		echo "  Binary not built yet"; \
	fi
	@echo ""
	@echo "  Go Version: $$(go version)"

# Check external dependencies
check-deps:
	@echo "📋 Checking external dependencies..."
	@echo ""
	@echo "Required for full functionality:"
	@which simed > /dev/null 2>&1 && echo "  ✓ simed found" || echo "  ✗ simed not found (required unless using --no-simed)"
	@which qrencode > /dev/null 2>&1 && echo "  ✓ qrencode found" || echo "  ✗ qrencode not found (optional, for QR codes)"
	@echo ""
	@echo "To install missing dependencies:"
	@echo "  simed:   (install from sim project)"
	@echo "  qrencode: brew install qrencode (macOS)"
	@echo "           apt-get install qrencode (Debian/Ubuntu)"

# Show help
help:
	@echo ""
	@echo "$(BINARY_NAME) - Self-contained Tailscale tunnel for Sim AI"
	@echo "============================================================"
	@echo ""
	@echo "Build targets:"
	@echo "  make              Build the binary (default)"
	@echo "  make build        Build the binary"
	@echo "  make build-debug  Build with debug symbols"
	@echo "  make build-all    Build for all platforms"
	@echo "  make clean        Remove build artifacts"
	@echo ""
	@echo "Run targets:"
	@echo "  make run          Build and run"
	@echo "  make run-verbose  Run with verbose logging"
	@echo "  make run-proxy    Run without starting simed"
	@echo ""
	@echo "Other targets:"
	@echo "  make deps         Download Go dependencies"
	@echo "  make test         Test the binary"
	@echo "  make install      Install to /usr/local/bin"
	@echo "  make uninstall    Remove from /usr/local/bin"
	@echo "  make info         Show binary information"
	@echo "  make check-deps   Check external dependencies"
	@echo "  make help         Show this help"
	@echo ""
	@echo "Usage examples:"
	@echo "  make              # Build the binary"
	@echo "  make run          # Build and run immediately"
	@echo "  make build-all    # Build for all platforms"
	@echo ""

.DEFAULT_GOAL := all
