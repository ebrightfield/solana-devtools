use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;

pub trait MutatedInstruction: Sized {
    fn update_program_id(self, program_id: Pubkey) -> Self;
    fn update_data(self, data: Vec<u8>) -> Self;
    fn update_account_meta_address(self, from: &Pubkey, to: Pubkey) -> Self;
    fn update_is_signer(self, pubkey: &Pubkey, is_signer: bool) -> Self;
    fn update_is_writable(self, pubkey: &Pubkey, is_writable: bool) -> Self;
    fn update_account_meta(self, pubkey: &Pubkey, account_meta: AccountMeta) -> Self;
}

impl MutatedInstruction for Instruction {
    fn update_program_id(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }

    fn update_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    fn update_account_meta_address(mut self, from: &Pubkey, to: Pubkey) -> Self {
        for account in &mut self.accounts {
            if account.pubkey == *from {
                account.pubkey = to;
            }
        }
        self
    }

    fn update_is_signer(mut self, pubkey: &Pubkey, is_signer: bool) -> Self {
        for account in &mut self.accounts {
            if account.pubkey == *pubkey {
                account.is_signer = is_signer;
            }
        }
        self
    }

    fn update_is_writable(mut self, pubkey: &Pubkey, is_writable: bool) -> Self {
        for account in &mut self.accounts {
            if account.pubkey == *pubkey {
                account.is_writable = is_writable;
            }
        }
        self
    }

    fn update_account_meta(mut self, pubkey: &Pubkey, account_meta: AccountMeta) -> Self {
        for account in &mut self.accounts {
            if account.pubkey == *pubkey {
                *account = account_meta;
                return self;
            }
        }
        self
    }
}
