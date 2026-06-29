# Baymaxed Tunnel

A self-contained, single-binary solution for creating secure Tailscale tunnels to Goose AI agents. This tool embeds Tailscale functionality directly using the `tsnet` library, eliminating the need for a separate `tailscaled` daemon.

## Features

- **Single Binary**: All functionality in one ~19MB executable
- **Embedded Tailscale**: Uses `tsnet` library for built-in Tailscale networking
- **Userspace Networking**: No root privileges required
- **Automatic Setup**: Handles authentication and network configuration
- **Flexible Modes**: Can start baymaxed or proxy to existing services
- **QR Code Support**: Generate QR codes for mobile client configuration
- **Cross-Platform**: Builds for Linux, macOS, and Windows
- **HTTPS Support**: Automatic TLS certificates via Tailscale

## Quick Start

```bash
# Clone the repository (or download the files)
git clone https://github.com/yourusername/baymaxed-tunnel.git
cd baymaxed-tunnel

# Build the binary
make

# Run it
./baymaxed-tunnel
```

## Installation

### From Source

```bash
# Download and build
git clone https://github.com/yourusername/baymaxed-tunnel.git
cd baymaxed-tunnel
make

# Optionally install system-wide
sudo make install
```

### Pre-built Binaries

Build for all platforms:

```bash
make build-all
# Binaries will be in the dist/ directory
```

## Usage

### Basic Usage

```bash
# Start tunnel with baymaxed
./baymaxed-tunnel

# Use a custom hostname
./baymaxed-tunnel --hostname my-baymax

# Proxy to existing service (don't start baymaxed)
./baymaxed-tunnel --no-baymaxed --port 8080

# Verbose mode for debugging
./baymaxed-tunnel --verbose
```

### Command-Line Options

```
--port PORT              Local port for baymaxed server (default: 62996)
--hostname NAME          Tailscale hostname (default: baymaxed-tunnel)
--state-dir PATH         Directory for Tailscale state (default: ~/.local/share/baymaxed-tunnel)
--no-baymaxed              Don't start baymaxed (proxy to existing service)
--baymaxed-path PATH       Path to baymaxed executable (default: baymaxed)
--no-qr                  Don't display QR code
--verbose                Enable verbose logging
--version                Show version and exit
```

### Examples

#### Proxy to Existing Service

If you already have a service running:

```bash
# Proxy to a service on port 8080
./baymaxed-tunnel --no-baymaxed --port 8080
```

#### Custom baymaxed Location

```bash
# Use baymaxed from a specific path
./baymaxed-tunnel --baymaxed-path /opt/baymax/bin/baymaxed
```

#### Different State Directory

```bash
# Use a custom state directory
./baymaxed-tunnel --state-dir /var/lib/baymaxed-tunnel
```

## How It Works

1. **Initialization**: Creates a Tailscale node using the embedded `tsnet` library
2. **Authentication**: If needed, provides an auth URL (opens automatically)
3. **Service Start**: Optionally starts baymaxed with a secure random key
4. **Proxy Setup**: Establishes HTTP/HTTPS reverse proxy from Tailscale to local service
5. **Connection Info**: Displays URLs and optionally generates QR code

## Building

### Requirements

- Go 1.23 or later
- Make (optional, but recommended)

### Build Commands

```bash
# Simple build
go build -o baymaxed-tunnel .

# Or using make
make                    # Build for current platform
make build-debug        # Build with debug symbols
make build-all          # Build for all platforms
make clean              # Clean build artifacts
```

### Build Targets

The Makefile supports building for:
- Linux (AMD64, ARM64)
- macOS (AMD64, ARM64/Apple Silicon)
- Windows (AMD64)

## Dependencies

### Build Dependencies

- Go 1.23+
- Internet connection (to download Go modules)

### Runtime Dependencies

#### Required (unless using --no-baymaxed)
- `baymaxed` - The Goose AI daemon

#### Optional
- `qrencode` - For QR code generation
  ```bash
  # macOS
  brew install qrencode
  
  # Linux
  apt-get install qrencode
  ```

### Check Dependencies

```bash
make check-deps
```

## State Management

The tunnel stores Tailscale state in:
- Default: `~/.local/share/baymaxed-tunnel/`
- Custom: Use `--state-dir` flag

This includes:
- Tailscale node keys
- Authentication state
- Network configuration

## Security

- **Encryption**: All traffic flows through Tailscale's encrypted WireGuard tunnels
- **Authentication**: Random secret keys generated for each baymaxed session
- **No Open Ports**: Services only accessible via Tailscale network
- **Userspace**: Runs entirely in userspace without root privileges

## Architecture

```
┌─────────────────┐
│  Mobile/Remote  │
│     Client      │
└────────┬────────┘
         │ HTTPS/HTTP
         ▼
┌─────────────────┐
│   Tailscale     │
│    Network      │
└────────┬────────┘
         │ WireGuard
         ▼
┌─────────────────┐
│  baymaxed-tunnel  │
│   (tsnet node)  │
├─────────────────┤
│  HTTP/S Proxy   │
└────────┬────────┘
         │ localhost
         ▼
┌─────────────────┐
│     baymaxed      │
│   (AI Agent)    │
└─────────────────┘
```

## Troubleshooting

### baymaxed not found

```bash
# Check if baymaxed is in PATH
which baymaxed

# Or specify path explicitly
./baymaxed-tunnel --baymaxed-path /path/to/baymaxed
```

### Authentication issues

1. Ensure you're logged into Tailscale
2. Check the auth URL provided
3. Verify network connectivity

### QR code not displaying

```bash
# Install qrencode
brew install qrencode  # macOS
apt-get install qrencode  # Debian/Ubuntu

# Or disable QR codes
./baymaxed-tunnel --no-qr
```

### Port already in use

```bash
# Use a different port
./baymaxed-tunnel --port 8080
```

## Development

### Project Structure

```
baymaxed-tunnel/
├── main.go           # All application code
├── go.mod            # Go module definition
├── go.sum            # Dependency checksums
├── Makefile          # Build automation
└── README.md         # This file
```

### Testing

```bash
# Test the build
make test

# Run with verbose logging
make run-verbose

# Check binary info
make info
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Comparison with Alternatives

| Feature | baymaxed-tunnel | tailscaled + launcher | ngrok |
|---------|--------------|----------------------|-------|
| Single Binary | ✅ 19MB | ❌ 30MB + launcher | ✅ |
| Self-Hosted | ✅ | ✅ | ❌ |
| No Account Limits | ✅ | ✅ | ❌ (free tier limited) |
| E2E Encryption | ✅ | ✅ | ✅ |
| Root Required | ❌ | ❌ | ❌ |
| Embedded Networking | ✅ | ❌ | N/A |

## License

[Add your license here]

## Acknowledgments

- Built with [Tailscale's tsnet](https://pkg.go.dev/tailscale.com/tsnet)
- Designed for [Goose AI](https://github.com/block/baymax)

## Support

For issues, questions, or contributions:
- Open an issue on GitHub
- Check existing issues for solutions
- Review the troubleshooting section above
