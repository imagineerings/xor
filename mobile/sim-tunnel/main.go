// simed-tunnel provides secure remote access to a Sim AI agent via Tailscale
package main

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"regexp"
	"strings"
	"sync"
	"syscall"
	"time"

	"tailscale.com/ipn/ipnstate"
	"tailscale.com/tsnet"
)

const (
	defaultPort = 62996
	appName     = "simed-tunnel"
	version     = "1.0.0"
)

// ANSI color codes for terminal output
const (
	red    = "\033[0;31m"
	green  = "\033[0;32m"
	yellow = "\033[1;33m"
	blue   = "\033[0;34m"
	cyan   = "\033[0;36m"
	nc     = "\033[0m" // No Color
)

// Config holds configuration for the tunnel
type Config struct {
	Port       int
	Hostname   string
	StateDir   string
	NoSimed   bool
	SimedPath string
	NoQR       bool
	Verbose    bool
}

// Tunnel manages the simed and Tailscale services
type Tunnel struct {
	config      *Config
	secret      string
	simedCmd   *exec.Cmd
	tsServer    *tsnet.Server
	homeDir     string
	authURLOnce sync.Once
}

func main() {
	config := parseFlags()

	if err := run(config); err != nil {
		log.Fatal(err)
	}
}

func parseFlags() *Config {
	config := &Config{}

	flag.IntVar(&config.Port, "port", defaultPort, "Local port for simed server")
	flag.StringVar(&config.Hostname, "hostname", "simed-tunnel", "Tailscale hostname")
	flag.StringVar(&config.StateDir, "state-dir", "", "Directory for Tailscale state (default: ~/.local/share/simed-tunnel)")
	flag.BoolVar(&config.NoSimed, "no-simed", false, "Don't start simed (proxy to existing service)")
	flag.StringVar(&config.SimedPath, "simed-path", "simed", "Path to simed executable")
	flag.BoolVar(&config.NoQR, "no-qr", false, "Don't display QR code")
	flag.BoolVar(&config.Verbose, "verbose", false, "Verbose logging")
	showVersion := flag.Bool("version", false, "Show version and exit")

	flag.Parse()

	if *showVersion {
		fmt.Printf("%s version %s\n", appName, version)
		os.Exit(0)
	}

	return config
}

func run(config *Config) error {
	tunnel := &Tunnel{
		config: config,
	}

	// Get home directory
	var err error
	tunnel.homeDir, err = os.UserHomeDir()
	if err != nil {
		return fmt.Errorf("failed to get home directory: %w", err)
	}

	// Set default state directory if not specified
	if config.StateDir == "" {
		config.StateDir = filepath.Join(tunnel.homeDir, ".local", "share", appName)
	}

	// Generate random secret
	if err := tunnel.generateSecret(); err != nil {
		return fmt.Errorf("failed to generate secret: %w", err)
	}

	// Print header
	tunnel.printHeader()

	// Check dependencies
	if err := tunnel.checkDependencies(); err != nil {
		return err
	}

	// Setup signal handling
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-sigChan
		fmt.Printf("\n%sShutting down...%s\n", yellow, nc)
		cancel()
	}()

	// Start simed if not disabled
	if !config.NoSimed {
		if err := tunnel.startSimed(ctx); err != nil {
			return err
		}
		defer tunnel.cleanup()
	} else {
		fmt.Printf("%sSkipping simed startup (--no-simed flag)%s\n", yellow, nc)
		fmt.Printf("%sProxying to existing service on port %d%s\n", cyan, config.Port, nc)
	}

	// Setup and start Tailscale using tsnet
	if err := tunnel.setupTailscale(ctx); err != nil {
		return err
	}

	// Get connection info
	v4, v6, host, err := tunnel.getConnectionInfo(ctx)
	if err != nil {
		return err
	}

	// Setup HTTP proxy to simed
	if err := tunnel.setupHTTPProxy(ctx); err != nil {
		return err
	}

	// Generate and display QR code
	if !config.NoQR {
		if err := tunnel.displayQRCode(v4, host); err != nil {
			// Non-fatal: just warn if QR code fails
			fmt.Printf("%sWarning: QR code generation failed: %v%s\n", yellow, err, nc)
		}
	}

	// Display connection info
	tunnel.displayConnectionInfo(v4, v6, host)

	// Keep running
	fmt.Printf("\n%s✓ Everything is running!%s\n", green, nc)
	fmt.Printf("%sPress Ctrl+C to stop%s\n\n", yellow, nc)

	<-ctx.Done()
	return nil
}

func (t *Tunnel) generateSecret() error {
	// Generate 24 bytes, base64 encode, remove special chars, take first 32 chars
	b := make([]byte, 24)
	if _, err := rand.Read(b); err != nil {
		return err
	}

	encoded := base64.StdEncoding.EncodeToString(b)
	// Remove =, +, / characters for URL safety
	encoded = strings.Map(func(r rune) rune {
		if r == '=' || r == '+' || r == '/' {
			return -1
		}
		return r
	}, encoded)

	if len(encoded) > 32 {
		encoded = encoded[:32]
	}
	t.secret = encoded
	return nil
}

func (t *Tunnel) printHeader() {
	fmt.Printf("%s╔════════════════════════════════════════════════════════════════════╗%s\n", blue, nc)
	fmt.Printf("%s║                      Simed Tunnel v%s                          ║%s\n", blue, version, nc)
	fmt.Printf("%s║                 Secure Tailscale Access to Sim AI                ║%s\n", blue, nc)
	fmt.Printf("%s╚════════════════════════════════════════════════════════════════════╝%s\n\n", blue, nc)
}

func (t *Tunnel) checkDependencies() error {
	// Check for simed if we're going to start it
	if !t.config.NoSimed {
		if _, err := exec.LookPath(t.config.SimedPath); err != nil {
			fmt.Printf("%sError: %s not found%s\n", red, t.config.SimedPath, nc)
			fmt.Printf("%sPlease ensure simed is installed or use --no-simed flag%s\n", yellow, nc)
			fmt.Printf("%sYou can also specify a custom path with --simed-path%s\n", yellow, nc)
			return fmt.Errorf("simed not found")
		}
	}

	// Check for qrencode if we're going to use it
	if !t.config.NoQR {
		if _, err := exec.LookPath("qrencode"); err != nil {
			fmt.Printf("%sWarning: qrencode not found%s\n", yellow, nc)
			fmt.Printf("%sQR code will not be displayed. Install with:%s\n", yellow, nc)
			fmt.Printf("%s  macOS:  brew install qrencode%s\n", cyan, nc)
			fmt.Printf("%s  Linux:  apt-get install qrencode%s\n", cyan, nc)
			fmt.Printf("%sOr use --no-qr to suppress this warning%s\n", yellow, nc)
			// Don't fail, just disable QR
			t.config.NoQR = true
		}
	}

	return nil
}

func (t *Tunnel) startSimed(ctx context.Context) error {
	fmt.Printf("%sStarting simed on port %d...%s\n", green, t.config.Port, nc)

	t.simedCmd = exec.CommandContext(ctx, t.config.SimedPath, "agent")
	t.simedCmd.Env = append(os.Environ(),
		fmt.Sprintf("GOOSE_PORT=%d", t.config.Port),
		fmt.Sprintf("GOOSE_SERVER__SECRET_KEY=%s", t.secret),
	)

	if t.config.Verbose {
		t.simedCmd.Stdout = os.Stdout
		t.simedCmd.Stderr = os.Stderr
	}

	if err := t.simedCmd.Start(); err != nil {
		return fmt.Errorf("failed to start simed: %w", err)
	}

	// Wait for simed to be ready
	fmt.Println("Waiting for simed to start...")
	healthURL := fmt.Sprintf("http://localhost:%d/health", t.config.Port)

	for i := 0; i < 30; i++ {
		resp, err := http.Get(healthURL)
		fmt.Println("Checking status of simed...")
		fmt.Println(resp)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusUnauthorized {
				fmt.Printf("%s✓ Simed is running (PID: %d)%s\n", green, t.simedCmd.Process.Pid, nc)
				return nil
			}
		}
		time.Sleep(500 * time.Millisecond)
	}

	return fmt.Errorf("simed failed to start within timeout")
}

func (t *Tunnel) setupTailscale(ctx context.Context) error {
	fmt.Printf("%sSetting up Tailscale (embedded via tsnet)...%s\n", green, nc)

	// Create state directory for tsnet
	if err := os.MkdirAll(t.config.StateDir, 0755); err != nil {
		return fmt.Errorf("failed to create state directory: %w", err)
	}

	// Set up custom logger that intercepts auth URL messages
	// Pattern to match: "To start this tsnet server, restart with TS_AUTHKEY set, or go to: https://..."
	authURLPattern := regexp.MustCompile(`go to: (https://login\.tailscale\.com/[^\s]+)`)

	customLogf := func(format string, args ...any) {
		msg := fmt.Sprintf(format, args...)
		
		// Check if this message contains an auth URL
		if matches := authURLPattern.FindStringSubmatch(msg); len(matches) > 1 {
			authURL := matches[1]
			
			// Only show the auth message once
			t.authURLOnce.Do(func() {
				fmt.Printf("\n%s🌐 Authentication required!%s\n", yellow, nc)
				fmt.Printf("%sPlease visit:%s %s%s\n", yellow, nc, authURL, nc)
				fmt.Printf("%s(Opening in your browser automatically...)%s\n\n", cyan, nc)
				
				// Try to open the URL in the default browser
				go openBrowser(authURL)
			})
			
			// Don't print the original log message since we've shown our own
			return
		}
		
		// Print all other messages (always, to match default behavior)
		log.Print(msg)
	}

	// Create tsnet server with custom logger
	t.tsServer = &tsnet.Server{
		Dir:      t.config.StateDir,
		Hostname: t.config.Hostname,
		Logf:     customLogf,
	}

	fmt.Println("▶️  Starting embedded Tailscale...")

	// Start tsnet server in the background
	go func() {
		// This will block until we have an IP address
		status, err := t.tsServer.Up(ctx)
		if err != nil {
			if t.config.Verbose {
				log.Printf("Failed to bring up tsnet: %v", err)
			}
			return
		}

		// Check if we need authentication (backup mechanism)
		if status.AuthURL != "" {
			t.authURLOnce.Do(func() {
				fmt.Printf("\n%s🌐 Authentication required!%s\n", yellow, nc)
				fmt.Printf("%sPlease visit:%s %s%s\n", yellow, nc, status.AuthURL, nc)
				fmt.Printf("%s(Opening in your browser automatically...)%s\n\n", cyan, nc)

				// Try to open the URL in the default browser
				openBrowser(status.AuthURL)
			})
		}
	}()

	// Give tsnet a moment to initialize
	time.Sleep(2 * time.Second)

	fmt.Printf("%s✓ Tailscale embedded server initialized%s\n", green, nc)
	return nil
}

func (t *Tunnel) getConnectionInfo(ctx context.Context) (v4, v6, host string, err error) {
	fmt.Printf("%sWaiting for Tailscale connection...%s\n", green, nc)

	// Wait for tsnet to be fully up and get status
	var status *ipnstate.Status
	for i := 0; i < 60; i++ { // Wait up to 60 seconds
		lc, err := t.tsServer.LocalClient()
		if err == nil {
			status, err = lc.Status(ctx)
			if err == nil && status.Self != nil && len(status.Self.TailscaleIPs) > 0 {
				break
			}
		}
		if i%5 == 0 && i > 0 {
			fmt.Printf("  Still waiting for Tailscale to connect... (%ds)\n", i)
		}
		time.Sleep(time.Second)
	}

	if status == nil || status.Self == nil {
		return "", "", "", fmt.Errorf("failed to get Tailscale status")
	}

	// Get DNS name
	if status.Self.DNSName != "" {
		host = strings.TrimSuffix(status.Self.DNSName, ".")
	}

	// Get IP addresses
	for _, ip := range status.Self.TailscaleIPs {
		if ip.Is4() && v4 == "" {
			v4 = ip.String()
		} else if ip.Is6() && v6 == "" {
			v6 = ip.String()
		}
	}

	if v4 == "" && v6 == "" {
		return "", "", "", fmt.Errorf("no Tailscale IP addresses available")
	}

	fmt.Printf("%s✓ Connected to Tailscale network%s\n", green, nc)
	return v4, v6, host, nil
}

func (t *Tunnel) setupHTTPProxy(ctx context.Context) error {
	fmt.Printf("%sSetting up HTTP proxy (Tailscale → localhost:%d)...%s\n", green, t.config.Port, nc)

	// Create a reverse proxy to forward requests to simed
	targetURL, err := url.Parse(fmt.Sprintf("http://localhost:%d", t.config.Port))
	if err != nil {
		return fmt.Errorf("failed to parse target URL: %w", err)
	}

	proxy := httputil.NewSingleHostReverseProxy(targetURL)

	// Add custom error handling
	proxy.ErrorHandler = func(w http.ResponseWriter, r *http.Request, err error) {
		if t.config.Verbose {
			log.Printf("Proxy error: %v", err)
		}
		w.WriteHeader(http.StatusBadGateway)
		fmt.Fprintf(w, "Proxy error: %v", err)
	}

	// Start HTTP server on tsnet
	go func() {
		ln, err := t.tsServer.Listen("tcp", ":80")
		if err != nil {
			log.Printf("Failed to listen on tsnet: %v", err)
			return
		}
		defer ln.Close()

		server := &http.Server{
			Handler:     proxy,
			ReadTimeout: 30 * time.Second,
		}

		fmt.Printf("%s✓ HTTP proxy established (port 80 → localhost:%d)%s\n", green, t.config.Port, nc)

		if err := server.Serve(ln); err != nil && err != http.ErrServerClosed {
			if t.config.Verbose {
				log.Printf("HTTP server error: %v", err)
			}
		}
	}()

	// Note: HTTPS support could be added here in future versions
	// when tsnet exposes TLS configuration methods

	return nil
}

func (t *Tunnel) displayConnectionInfo(v4, v6, host string) {
	fmt.Println()
	fmt.Printf("%s╔════════════════════════════════════════════════════════════════════╗%s\n", blue, nc)
	fmt.Printf("%s║                     Connection Information                         ║%s\n", blue, nc)
	fmt.Printf("%s╚════════════════════════════════════════════════════════════════════╝%s\n\n", blue, nc)

	if host != "" {
		fmt.Printf("%sTailscale URL:%s  http://%s\n", green, nc, host)
	}
	if v4 != "" {
		fmt.Printf("%sIPv4 Address:%s   %s\n", green, nc, v4)
		fmt.Printf("%s             %s   http://%s\n", green, nc, v4)
	}
	if v6 != "" {
		fmt.Printf("%sIPv6 Address:%s   [%s]\n", green, nc, v6)
		fmt.Printf("%s             %s   http://[%s]\n", green, nc, v6)
	}

	if !t.config.NoSimed {
		fmt.Printf("%sSecret Key:%s     %s\n", green, nc, t.secret)
	}
	fmt.Printf("%sLocal Port:%s     %d\n", green, nc, t.config.Port)
	fmt.Printf("%sState Dir:%s      %s\n", green, nc, t.config.StateDir)
}

func (t *Tunnel) displayQRCode(v4, host string) error {
	tunnelURL := fmt.Sprintf("http://%s", v4)
	if host != "" {
		tunnelURL = fmt.Sprintf("http://%s", host)
	}

	// Create configuration JSON
	configJSON := map[string]string{
		"url":    tunnelURL,
		"secret": t.secret,
	}
	configData, err := json.Marshal(configJSON)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	// URL encode the config
	urlEncodedConfig := url.QueryEscape(string(configData))

	// Create app URL for deep linking
	appURL := fmt.Sprintf("simchat://configure?data=%s", urlEncodedConfig)

	fmt.Println()
	fmt.Printf("%s╔════════════════════════════════════════════════════════════════════╗%s\n", blue, nc)
	fmt.Printf("%s║                          QR Code (Scan Me!)                        ║%s\n", blue, nc)
	fmt.Printf("%s╚════════════════════════════════════════════════════════════════════╝%s\n\n", blue, nc)

	// Generate QR code
	cmd := exec.Command("qrencode", "-t", "ANSIUTF8", appURL)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to generate QR code: %w", err)
	}

	fmt.Println()
	fmt.Printf("%s────────────────────────────────────────────────────────────────────%s\n", blue, nc)
	fmt.Printf("%sApp URL:%s %s\n", yellow, nc, appURL)
	fmt.Printf("%s────────────────────────────────────────────────────────────────────%s\n\n", blue, nc)

	return nil
}

func (t *Tunnel) cleanup() {
	fmt.Printf("\n%sCleaning up...%s\n", yellow, nc)

	if t.simedCmd != nil && t.simedCmd.Process != nil {
		fmt.Printf("Stopping simed (PID: %d)\n", t.simedCmd.Process.Pid)
		t.simedCmd.Process.Signal(os.Interrupt)
		// Give it a moment to clean up gracefully
		done := make(chan error, 1)
		go func() {
			done <- t.simedCmd.Wait()
		}()
		select {
		case <-done:
			// Process ended gracefully
		case <-time.After(5 * time.Second):
			// Force kill if it doesn't stop
			t.simedCmd.Process.Kill()
		}
	}

	if t.tsServer != nil {
		fmt.Println("Closing Tailscale connection...")
		t.tsServer.Close()
	}
}

// openBrowser tries to open a URL in the default browser
func openBrowser(url string) {
	var cmd *exec.Cmd
	switch os := getOS(); os {
	case "darwin":
		cmd = exec.Command("open", url)
	case "linux":
		// Try xdg-open first, fallback to others
		if _, err := exec.LookPath("xdg-open"); err == nil {
			cmd = exec.Command("xdg-open", url)
		} else if _, err := exec.LookPath("gnome-open"); err == nil {
			cmd = exec.Command("gnome-open", url)
		} else if _, err := exec.LookPath("kde-open"); err == nil {
			cmd = exec.Command("kde-open", url)
		}
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", url)
	}

	if cmd != nil {
		cmd.Run()
	}
}

// getOS returns the operating system name
func getOS() string {
	switch runtime := os.Getenv("GOOS"); runtime {
	case "":
		// GOOS not set, check runtime
		if strings.Contains(strings.ToLower(os.Getenv("OS")), "windows") {
			return "windows"
		}
		// Try uname
		if out, err := exec.Command("uname", "-s").Output(); err == nil {
			s := strings.ToLower(strings.TrimSpace(string(out)))
			if s == "darwin" {
				return "darwin"
			}
			return "linux"
		}
		return "unknown"
	default:
		return runtime
	}
}
