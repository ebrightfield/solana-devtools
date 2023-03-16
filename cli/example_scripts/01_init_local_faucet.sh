#!/bin/sh

source example_scripts/config.sh

set -x

solana-keygen new -o "local_faucet_mint.json" --no-passphrase
solana-keygen new -o "local_faucet.json" --no-passphrase

cargo build --bin solana-devtools

echo Initializing mint

spl-faucet init-spl-mint local_faucet_mint.json $DECIMALS

echo Sleeping to confirm transaction...
sleep 16
echo Initializing faucet

spl-faucet init-faucet local_faucet_mint.json $MAX_AIRDROP local_faucet.json
