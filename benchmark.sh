#!/usr/bin/env bash

cargo bench && \
cargo run --example save_to_file resources/tests/large_100/ current_large_100.sst && \
cargo run --example save_to_file resources/tests/large_1000/ current_large_1000.sst

# Display the file sizes
ls -lh ./

