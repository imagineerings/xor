# Cli

## Testing

You can test your changes to the `cli` crate by first building the main baymax binary:

```
cargo build -p baymax
```

And then building and running the `cli` crate with the following parameters:

```
 cargo run -p cli -- --baymax ./target/debug/baymax.exe
```
