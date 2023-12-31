use solana_devtools_signers::ConcreteSigner;
use solana_sdk::signer::Signer;
use std::str::FromStr;

fn main() {
    let signer = ConcreteSigner::from_str("usb://ledger?key=0").unwrap();
    println!("{}", signer.pubkey().to_string());
}
