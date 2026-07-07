---
title: Building Sim for FreeBSD
description: "Guide to building sim for freebsd for Sim development."
---

# Building Sim for FreeBSD

FreeBSD is not currently a supported platform, so this guide is a work in progress.

## Repository

Clone the [Sim repository](https://github.com/simtropolis/sim).

## Dependencies

- Install the necessary system packages and rustup:

  ```sh
  script/freebsd
  ```

  If preferred, you can inspect [`script/freebsd`](https://github.com/simtropolis/sim/blob/main/script/freebsd) and perform the steps manually.

## Building from source

Once the dependencies are installed, you can build Sim using [Cargo](https://doc.rust-lang.org/cargo/).

For a debug build of the editor:

```sh
cargo run
```

And to run the tests:

```sh
cargo test --workspace
```

In release mode, the primary user interface is the `cli` crate. You can run it in development with:

```sh
cargo run -p cli
```

### WebRTC Notice

Building `webrtc-sys` on FreeBSD currently fails due to missing upstream support and unavailable prebuilt binaries. As a result, collaboration features that depend on WebRTC (audio calls and screen sharing) are temporarily disabled.

See [Issue #15309: FreeBSD Support] and [Discussion #29550: Unofficial FreeBSD port for Sim] for more.

## Troubleshooting

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.
