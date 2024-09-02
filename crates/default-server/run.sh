#!/bin/bash -xeu
YEWOH_POSTGRES=postgres://postgres:postgres@localhost/anon RUST_LOG=info cargo run
