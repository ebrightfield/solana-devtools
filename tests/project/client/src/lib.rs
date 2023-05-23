
use test_program::instruction;
use test_program::accounts;
use solana_devtools_tx::anchor_instruction::to_anchor_instruction;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;

pub struct TestProgramClient {
    program_id: Pubkey,
}

impl TestProgramClient {
    fn initialize(&self) -> Instruction {
        to_anchor_instruction::<_,
            instruction::Initialize,
            accounts::Initialize
        >(&self, self.program_id)
    }
}

impl Into<instruction::Initialize> for &TestProgramClient {
    fn into(self) -> instruction::Initialize {
        instruction::Initialize {}
    }
}

impl Into<accounts::Initialize> for &TestProgramClient {
    fn into(self) -> accounts::Initialize {
        accounts::Initialize {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ix() {
        let test_prog_client = TestProgramClient { program_id: Pubkey::default() };
        let ix: Instruction = test_prog_client.initialize();
    }
}
