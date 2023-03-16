#!/bin/sh

# Localnet loaded with faucet program

solana-test-validator --bpf-program 4bXpkKSV8swHSnwqtzuboGPaPDeEgAn4Vt8GfarV5rZt faucet/lib/faucet.so $@
