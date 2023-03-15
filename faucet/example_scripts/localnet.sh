#!/bin/sh

# Localnet loaded with faucet program

solana-test-validator --reset --bpf-program 4bXpkKSV8swHSnwqtzuboGPaPDeEgAn4Vt8GfarV5rZt $(dirname $0)/../lib/faucet.so
