# Collaborative Workspace host-window evidence

The deterministic `actual-screenshot-2.png` capture is GPUI content raster output. It deliberately does not contain the macOS-owned traffic-light controls. The visual runner separately asserts that `PlatformTitleBar` owns the title-bar surface.

For full-window evidence, grant the terminal Screen Recording permission, capture the visible Rust-product window through the macOS screenshot UI, and rerun with `COLLABORATIVE_NATIVE_WINDOW_CAPTURE=/absolute/path/to/capture.png`. The runner validates and copies that native capture to `host-window-screenshot-2.png`; it never synthesizes window controls.
