
use solana_sdk::signer::Signer;
use solana_devtools_signers::ConcreteSigner;

fn main() {
    let signer = ConcreteSigner::new("usb://ledger?key=0").unwrap();
    println!("{}", signer.pubkey().to_string());
}