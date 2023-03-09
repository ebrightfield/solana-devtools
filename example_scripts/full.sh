#!/bin/bash
# This will run some CLI commands to create a faucet on devnet
# using your user's signer in `~/.config/solana/id.json`.
./example_scripts/01_init_local_faucet.sh
echo

echo Sleeping to confirm transaction...

sleep 16

./example_scripts/02_airdrop_local_faucet.sh
echo

# Wait and check our token balance
echo Sleeping then checking token balance...
sleep 5
echo This balance should be non-zero:
spl-token balance local_faucet_mint.json
