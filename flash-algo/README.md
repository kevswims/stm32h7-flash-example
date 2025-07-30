# Flash Algorithm Template

This is a flash algorithm template for writing CMSIS-Pack flash algorithms in Rust.
It can be used to generate new flash algorithms for usage with `probe-rs`.

## Dependencies

Run the following requirements:

```bash
cargo install cargo-generate cargo-binutils target-gen
rustup component add llvm-tools-preview
```

## Developing the algorithm

Just run `cargo run`. It spits out the flash algo in the probe-rs YAML format and downloads it onto a target and makes a test run.
You will also be able to see RTT messages.

You can find the generated YAML in `target/definition.yaml`.

Note: The test run from `cargo run` seems to always succeed even if it is broken.
