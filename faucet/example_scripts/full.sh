#!/bin/bash

# Create a new mint and a faucet for it
./example_scripts/01_init_local_faucet.sh
echo

echo Sleeping to confirm transaction...

sleep 16

# Use the new faucet, airdrop tokens a couple times
./example_scripts/02_airdrop_local_faucet.sh
echo

echo Sleeping then checking token balance...

sleep 5

echo This balance should be non-zero:

# Check token balance
spl-token balance local_faucet_mint.json
