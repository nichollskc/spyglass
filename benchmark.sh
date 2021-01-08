#!/usr/bin/env bash

# Clone the repository
REMOTE_URL="$(git config --get remote.origin.url)";
cd ${TRAVIS_BUILD_DIR}/.. && \
git clone ${REMOTE_URL} "${TRAVIS_REPO_SLUG}-bench" && \
cd  "${TRAVIS_REPO_SLUG}-bench" && \

# Bench master
git checkout master && \
cargo bench -- --save-baseline master && \
cargo run --example save_to_file resources/tests/large_100/ master_large_100.sst && \
cargo run --example save_to_file resources/tests/full/ master_full.sst && \

# Bench current branch
git checkout ${TRAVIS_COMMIT}^1 && \
cargo bench -- --save-baseline before && \
cargo run --example save_to_file resources/tests/large_100/ before_large_100.sst && \
cargo run --example save_to_file resources/tests/full/ before_full.sst && \

# Bench current branch
git checkout ${TRAVIS_COMMIT} && \
cargo bench -- --save-baseline current && \
cargo run --example save_to_file resources/tests/large_100/ current_large_100.sst && \
cargo run --example save_to_file resources/tests/full/ current_full.sst && \

# Install https://github.com/BurntSushi/critcmp
cargo install critcmp --force && \

# Compare the two generated benches
critcmp master before current && \

# Display the file sizes
ls -lh ./

