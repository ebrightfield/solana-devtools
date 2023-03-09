#!/bin/sh

set +x

KEYPAIR="/home/eric/.config/solana/id.json"
AMOUNT=100000
NETWORK=devnet

solana config set -u $NETWORK -k $KEYPAIR

echo "Attempting to airdrop, first attempt might create ATA instead."
echo

target/debug/spl-faucet mint local_faucet.json $AMOUNT
echo
echo "Sleeping then executing another airdrop transaction"
sleep 16
target/debug/spl-faucet mint local_faucet.json $AMOUNT
