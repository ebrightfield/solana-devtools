#!/bin/sh

# building the program because this localnet configuration loads it
anchor build

cargo run --bin localnet_suite_1 -- build-json --overwrite-existing
cargo run --bin localnet_suite_1 -- test-validator -- --reset

# In another terminal, you can query for loaded accounts and programs:

# spl-token -ul display 9WQV5oLq9ykMrqSj6zWrazr3SjFzbESXcVwZYttsd7XM
# solana -ul program show Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS