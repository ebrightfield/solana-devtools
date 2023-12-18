use lazy_static::lazy_static;
use solana_program::bpf_loader_deprecated;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::bpf_loader;
use solana_sdk::bpf_loader_upgradeable;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

pub struct SplProgram {
    address: Pubkey,
    data: Vec<u8>,
    executable: bool,
    owner: Pubkey,
}

impl Into<Account> for &SplProgram {
    fn into(self) -> Account {
        Account {
            lamports: 1,
            data: self.data.clone(),
            owner: self.owner,
            executable: self.executable,
            rent_epoch: 1,
        }
    }
}

impl Into<(Pubkey, AccountSharedData)> for &SplProgram {
    fn into(self) -> (Pubkey, AccountSharedData) {
        let act: Account = self.into();
        (self.address, act.into())
    }
}

lazy_static! {
    pub static ref SPL_PROGRAMS: Vec<SplProgram> = vec![
        SplProgram {
            address: pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
            data: include_bytes!("associated_token_program.so").to_vec(),
            executable: true,
            owner: bpf_loader::ID,
        },
        SplProgram {
            address: pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
            data: include_bytes!("token_program.so").to_vec(),
            executable: true,
            owner: pubkey!("BPFLoader2111111111111111111111111111111111"),
        },
        SplProgram {
            address: pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
            data: include_bytes!("token-2022.so").to_vec(),
            executable: true,
            owner: bpf_loader_upgradeable::ID,
        },
        SplProgram {
            address: pubkey!("DoU57AYuPFu2QU514RktNPG22QhApEjnKxnBcu4BHDTY"),
            data: include_bytes!("token-2022-data.so").to_vec(),
            executable: false,
            owner: bpf_loader_upgradeable::ID,
        },
        SplProgram {
            address: pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo"),
            data: include_bytes!("memo-v1.so").to_vec(),
            executable: true,
            owner: bpf_loader::ID,
        },
        SplProgram {
            address: pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
            data: include_bytes!("memo.so").to_vec(),
            executable: true,
            owner: bpf_loader_deprecated::ID,
        },
    ];
}
