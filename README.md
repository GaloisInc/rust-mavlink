# rust-mavlink

Rust implementation of the [MAVLink](http://qgroundcontrol.org/mavlink/start) UAV messaging protocol,
with bindings for the [common message set](http://mavlink.org/messages/common).

Add to your Cargo.toml:

```
mavlink = "0.4"
```

See [src/bin/mavlink-dump.rs](src/bin/mavlink-dump.rs) for a usage example.

## Quickcheck
TODO

## Fuzzing
We are using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) for fuzz testing. To install it do:

```
$ cargo install cargo-fuzz
```

And to run the fuzz-test do:
```
cargo fuzz run mavlink_v1
# or
cargo fuzz run mavlink_v2
```

More details about [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) can be found in the [fuzz-testing book](https://rust-fuzz.github.io/book/introduction.html)

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.
