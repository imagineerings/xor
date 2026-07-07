# Cli

## Testing

You can test your changes to the `cli` crate by first building the main sim binary:

```
cargo build -p sim
```

And then building and running the `cli` crate with the following parameters:

```
 cargo run -p cli -- --sim ./target/debug/sim.exe
```
