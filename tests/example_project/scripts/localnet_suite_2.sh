#!/bin/sh

cargo run --bin localnet_suite_2 -- build-json --overwrite-existing
cargo run --bin localnet_suite_2 -- test-validator -- --reset

