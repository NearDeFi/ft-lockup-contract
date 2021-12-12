#!/bin/bash
set -e
pushd "$(dirname $0)"

# Removing rlib for contract building
perl -i -pe 's/\["cdylib", "rlib"\]/\["cdylib"\]/' Cargo.toml

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
mkdir -p ./res
cp target/wasm32-unknown-unknown/release/ft_lockup.wasm ./res/

# Restoring rlib
perl -i -pe 's/\["cdylib"\]/\["cdylib", "rlib"\]/' Cargo.toml
popd
