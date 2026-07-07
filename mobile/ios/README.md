# Sim for mobile

A native iOS client for the Sim AI Assistant that communicates with the simed server.

*Supporting infrastructure*

This makes use of the following related infrastructure:

## Tunnels for access to sim agents

For access to personally run sim agents (ie on a desktop or server that isn't directly addressable) you will need a tunnel.

### Cloudflare DO powered tunnel
To allow access from behind NAT, https://github.com/michaelneale/lapstone-tunnel is used by default.
This opens an outbound websocket connection, and provides a stable https url for the mobile clients to connect through to.

### Tailscale tunnel
`tailscale` is also an option here, this can be chosen when launching via the desktop app (or script) and will require tailscale client on ios app (currently) and that you are logged in in both cases. The advantage of tailscale is additional layer of security, and optimal peer-to-peer connections when the network allows for low latency.

# Trial service (default config)

There is a demo grade "simed" service hosted on fly.io which is default value for the app when installed.
This only supports limited functionality, and one session per device, which is ephemeral (and limited tools).

Code for this: https://github.com/michaelneale/demo-simed-fly.io

This is not intended for production use but just so app works with zero config.

# Trying it out

Some ways to run this below

## Run the sim app in emulator

* run `./launch_simd.sh` - it will show you url and secret to use (needs simed binary from sim on your $PATH)
use that in the simulator app
*

## Running from test flight on real phone

(only attempt if familiar with ios dev)

* install sim from testflight
* run `curl -sSL https://raw.githubusercontent.com/michaelneale/sim-ios/main/launch_tunnel.sh | bash` - this runs the helper script to stand up a tunnel or `just run-ui` from the sim branch: micn/sim-mobile-access and enable the tunnel from the app settings
* point your phone at the QR code to configure it

## Public beta

https://testflight.apple.com/join/hsRdwbS3
