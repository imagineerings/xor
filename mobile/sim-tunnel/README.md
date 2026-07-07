# Simed Tunnel

A self-contained, single-binary solution for creating secure Tailscale tunnels to Goose AI agents. This tool embeds Tailscale functionality directly using the `tsnet` library, eliminating the need for a separate `tailscaled` daemon.

## Features

- **Single Binary**: All functionality in one ~19MB executable
- **Embedded Tailscale**: Uses `tsnet` library for built-in Tailscale networking
- **Userspace Networking**: No root privileges required
- **Automatic Setup**: Handles authentication and network configuration
- **Flexible Modes**: Can start simed or proxy to existing services
- **QR Code Support**: Generate QR codes for mobile client configuration
- **Cross-Platform**: Builds for Linux, macOS, and Windows
- **HTTPS Support**: Automatic TLS certificates via Tailscale

## Quick Start

```bash
# Clone the repository (or download the files)
git clone https://github.com/yourusername/simed-tunnel.git
cd simed-tunnel

# Build the binary
make

# Run it
./simed-tunnel
```

## Installation

### From Source

```bash
# Download and build
git clone https://github.com/yourusername/simed-tunnel.git
cd simed-tunnel
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
# Start tunnel with simed
./simed-tunnel

# Use a custom hostname
./simed-tunnel --hostname my-sim

# Proxy to existing service (don't start simed)
./simed-tunnel --no-simed --port 8080

# Verbose mode for debugging
./simed-tunnel --verbose
```

### Command-Line Options

```
--port PORT              Local port for simed server (default: 62996)
--hostname NAME          Tailscale hostname (default: simed-tunnel)
--state-dir PATH         Directory for Tailscale state (default: ~/.local/share/simed-tunnel)
--no-simed              Don't start simed (proxy to existing service)
--simed-path PATH       Path to simed executable (default: simed)
--no-qr                  Don't display QR code
--verbose                Enable verbose logging
--version                Show version and exit
```

### Examples

#### Proxy to Existing Service

If you already have a service running:

```bash
# Proxy to a service on port 8080
./simed-tunnel --no-simed --port 8080
```

#### Custom simed Location

```bash
# Use simed from a specific path
./simed-tunnel --simed-path /opt/sim/bin/simed
```

#### Different State Directory

```bash
# Use a custom state directory
./simed-tunnel --state-dir /var/lib/simed-tunnel
```

## How It Works

1. **Initialization**: Creates a Tailscale node using the embedded `tsnet` library
2. **Authentication**: If needed, provides an auth URL (opens automatically)
3. **Service Start**: Optionally starts simed with a secure random key
4. **Proxy Setup**: Establishes HTTP/HTTPS reverse proxy from Tailscale to local service
5. **Connection Info**: Displays URLs and optionally generates QR code

## Building

### Requirements

- Go 1.23 or later
- Make (optional, but recommended)

### Build Commands

```bash
# Simple build
go build -o simed-tunnel .

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

#### Required (unless using --no-simed)
- `simed` - The Goose AI daemon

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
- Default: `~/.local/share/simed-tunnel/`
- Custom: Use `--state-dir` flag

This includes:
- Tailscale node keys
- Authentication state
- Network configuration

## Security

- **Encryption**: All traffic flows through Tailscale's encrypted WireGuard tunnels
- **Authentication**: Random secret keys generated for each simed session
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
│  simed-tunnel  │
│   (tsnet node)  │
├─────────────────┤
│  HTTP/S Proxy   │
└────────┬────────┘
         │ localhost
         ▼
┌─────────────────┐
│     simed      │
│   (AI Agent)    │
└─────────────────┘
```

## Troubleshooting

### simed not found

```bash
# Check if simed is in PATH
which simed

# Or specify path explicitly
./simed-tunnel --simed-path /path/to/simed
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
./simed-tunnel --no-qr
```

### Port already in use

```bash
# Use a different port
./simed-tunnel --port 8080
```

## Development

### Project Structure

```
simed-tunnel/
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

| Feature | simed-tunnel | tailscaled + launcher | ngrok |
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
- Designed for [Goose AI](https://github.com/block/sim)

## Support

For issues, questions, or contributions:
- Open an issue on GitHub
- Check existing issues for solutions
- Review the troubleshooting section above
