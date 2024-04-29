use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;

/// Extends [Instruction]s to allow for mutating the program ID,
/// instruction data, or account meta pubkeys and signer / mutable flags.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_mutations() {
        let prog1 = Pubkey::new_unique();
        let prog2 = Pubkey::new_unique();
        let act1 = Pubkey::new_unique();
        let act2 = Pubkey::new_unique();
        let act3 = Pubkey::new_unique();

        let data1 = vec![0, 1, 2, 3];

        let acts = vec![AccountMeta::new(act1, false), AccountMeta::new(act2, true)];

        let ix = Instruction::new_with_bytes(prog1, &data1, acts);

        let mutated = ix.clone().update_program_id(prog2);
        assert_eq!(prog2, mutated.program_id);

        let mutated = ix.clone().update_is_writable(&act1, false);
        assert!(mutated.accounts[0].pubkey == act1 && !mutated.accounts[0].is_writable);

        let mutated = ix.clone().update_is_signer(&act1, true);
        assert!(mutated.accounts[0].pubkey == act1 && mutated.accounts[0].is_signer);

        let mutated = ix.clone().update_account_meta_address(&act1, act3);
        assert!(mutated.accounts[0].pubkey == act3);

        let mutated = ix
            .clone()
            .update_account_meta(&act2, AccountMeta::new_readonly(act3, false));
        assert!(
            mutated.accounts[1].pubkey == act3
                && !mutated.accounts[1].is_writable
                && !mutated.accounts[1].is_signer
        );

        let data2 = vec![4, 5, 6, 7];
        let mutated = ix.clone().update_data(data2.clone());
        assert_eq!(data2, mutated.data);
    }
}
