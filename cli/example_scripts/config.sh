#!/bin/sh

export DECIMALS=6
export KEYPAIR=~/.config/solana/id.json
export MAX_AIRDROP=10000000
export NETWORK=l
export AMOUNT=1000000

alias spl-faucet="../target/debug/solana-devtools faucet"

solana config set -u $NETWORK -k $KEYPAIR

