use solana_devtools_macros::named_pubkey;
use solana_sdk::pubkey::Pubkey;

#[test]
fn named_pubkeys() {
    const _: Pubkey = named_pubkey!("myname");
    const _: Pubkey = named_pubkey!("myname123");
    const _: Pubkey = named_pubkey!("myreallylongname123456789iiiiiii");
}
