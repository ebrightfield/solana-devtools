#!/bin/sh

set +x

DECIMALS=6
KEYPAIR="/home/eric/.config/solana/id.json"
MAX_AIRDROP=10000000
NETWORK=devnet

solana config set -u $NETWORK -k $KEYPAIR

solana-keygen new -o "local_faucet_mint.json" --no-passphrase
solana-keygen new -o "local_faucet.json" --no-passphrase

cargo build --bin spl-faucet

echo "Initializing mint"
target/debug/spl-faucet init-spl-mint local_faucet_mint.json $DECIMALS

echo "Sleeping to confirm transaction..."
sleep 16
echo "Initializing faucet"

target/debug/spl-faucet init-faucet local_faucet_mint.json $MAX_AIRDROP local_faucet.json
