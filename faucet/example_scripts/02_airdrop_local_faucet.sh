#!/bin/sh

source example_scripts/config.sh

set -x

echo "Airdropping"
echo

# The init-ata flag will add an instruction to create
# the signer's associated token account.
spl-faucet mint --init-ata local_faucet.json $AMOUNT

echo
echo "Sleeping then airdropping again"
sleep 16

spl-faucet mint local_faucet.json $AMOUNT
