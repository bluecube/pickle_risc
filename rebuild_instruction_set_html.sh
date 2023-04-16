#!/bin/sh
cd toolchain-rs
cargo run \
    --bin instruction_set_html_generator \
    -- \
    ../instruction_set.json5 \
    -o ../instruction_set.html
