use anchor_lang;
use anyhow::anyhow;
use base64::{engine::general_purpose::STANDARD, Engine};
use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Prefix of a program log inside an instruction.
const PROGRAM_LOG: &str = "Program log: ";
/// Prefix of a program log of an Event.
const PROGRAM_DATA: &str = "Program data: ";

lazy_static! {
    static ref CPI_PUSH_RE: Regex = Regex::new(r"^Program (.*) invoke.*$").unwrap();
    static ref CPI_POP_RE: Regex = Regex::new(r"^Program (.*) success*$").unwrap();
    static ref PROGRAM_FAILURE_RE: Regex = Regex::new(r"^Program (.*) failed: (.*)$").unwrap();
}

fn handle_program_log<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
    l: &str,
) -> anyhow::Result<(Option<T>, CpiStackManipulation)> {
    // Log emitted from the current program.
    if let Some(log) = l
        .strip_prefix(PROGRAM_LOG)
        .or_else(|| l.strip_prefix(PROGRAM_DATA))
    {
        let borsh_bytes = match STANDARD.decode(log) {
            Ok(borsh_bytes) => borsh_bytes,
            _ => {
                return Ok((None, CpiStackManipulation::None));
            }
        };

        let mut slice: &[u8] = &borsh_bytes[..];
        let disc: [u8; 8] = {
            let mut disc = [0; 8];
            disc.copy_from_slice(&borsh_bytes[..8]);
            slice = &slice[8..];
            disc
        };
        let mut event = None;
        if disc == T::discriminator() {
            let e: T = anchor_lang::AnchorDeserialize::deserialize(&mut slice).map_err(|e| {
                anyhow!(
                    "Error parsing event data that matched the discriminator: {}",
                    e.to_string()
                )
            })?;
            event = Some(e);
        }
        Ok((event, CpiStackManipulation::None))
    }
    // System log.
    else {
        let push_or_pop = handle_system_log(l);
        Ok((None, push_or_pop))
    }
}

/// Detect whether we have pushed or popped.
fn handle_system_log(log: &str) -> CpiStackManipulation {
    if CPI_PUSH_RE.is_match(log) {
        let c = CPI_PUSH_RE
            .captures(log)
            .expect("unable to parse system log");
        let program = c
            .get(1)
            .expect("unable to parse system log")
            .as_str()
            .to_string();
        CpiStackManipulation::Push(program)
    } else if CPI_POP_RE.is_match(log) {
        CpiStackManipulation::Pop
    } else {
        CpiStackManipulation::None
    }
}

/// Iterate through the logs of a transaction execution,
/// looking for logs originating from a target program (by pubkey string),
/// and return any events of type `T` logged, and error information if the transaction failed.
pub fn parse_transaction_logs<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
    logs: Vec<String>,
    target_program_id: &str,
) -> Vec<(String, T)> {
    let mut events = Vec::new();
    if !logs.is_empty() {
        if let Ok(mut execution) = ProgramCpiStack::new(&mut logs.as_ref()) {
            for l in logs {
                // Parse the program logs
                let (event, push_or_pop) = {
                    // Is the log part of the target program?
                    if !execution.is_empty() && target_program_id == execution.program() {
                        handle_program_log(&l).unwrap_or_else(|e| {
                            error!("Unable to parse log: {e}");
                            (None, CpiStackManipulation::None)
                        })
                    } else {
                        // If not, then see if we pushed or popped
                        let push_or_pop = handle_system_log(&l);
                        (None, push_or_pop)
                    }
                };
                // Emit the event.
                if let Some(e) = event {
                    events.push((l, e));
                }
                match push_or_pop {
                    CpiStackManipulation::Push(new_program) => execution.push(new_program),
                    CpiStackManipulation::Pop => execution.pop(),
                    _ => {}
                }
            }
        }
    }
    events
}

/// Tracks whether logs indicate a push or pop through the CPI stack.
#[derive(Debug, Clone, PartialEq)]
enum CpiStackManipulation {
    /// Stores the Program ID of the CPI invoked
    Push(String),
    Pop,
    None,
}

/// Tracks the current-running program as a transaction pushes and pops
/// up and down the call-stack with CPIs and instructions.
/// For example, if we're at top-level execution, there is one `String`
/// in `self.stack`, which is the base58 Program ID.
/// If we're one-level down in a CPI, there are two elements in the stack,
/// and the second element is the program ID invoked as a CPI.
struct ProgramCpiStack {
    stack: Vec<String>,
}

impl ProgramCpiStack {
    fn new(logs: &mut &[String]) -> anyhow::Result<Self> {
        let l = &logs[0];
        *logs = &logs[1..];

        // These should never fail
        // as long as we are processing the first Solana transaction log.
        let c = CPI_PUSH_RE.captures(l).ok_or(anyhow!(
            "Failed to parse a program ID from Solana program log: {}",
            l
        ))?;
        let program = c
            .get(1)
            .ok_or(anyhow!(
                "Failed to parse a program ID from Solana program log: {}",
                l
            ))?
            .as_str()
            .to_string();
        Ok(Self {
            stack: vec![program],
        })
    }

    fn program(&self) -> String {
        assert!(!self.stack.is_empty(), "{:?}", self.stack);
        self.stack[self.stack.len() - 1].clone()
    }

    fn push(&mut self, new_program: String) {
        self.stack.push(new_program);
    }

    fn pop(&mut self) {
        assert!(!self.stack.is_empty());
        self.stack.pop().unwrap();
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

/// When a transaction execution fails, then its final log
/// prints the program that failed, and an error message from a
/// `solana_program::instruction::InstructionError`.
#[derive(Debug, Clone, PartialEq)]
pub struct LoggedTransactionFailure {
    pub program: Pubkey,
    /// `Display` of a `solana_program::instruction::InstructionError`.
    pub error: String,
}

/// An example might be:
/// "Program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 failed: custom program error: 0x1771"
/// Where the second string comes from the `Display` of a
/// `solana_program::instruction::InstructionError`.
pub fn check_for_program_error(log: &str) -> Option<LoggedTransactionFailure> {
    PROGRAM_FAILURE_RE.captures(log).map(|c| {
        let program = c.get(1).unwrap();
        let program = Pubkey::from_str(program.as_str()).unwrap();
        let error = c.get(2).unwrap().as_str().to_string();
        LoggedTransactionFailure { program, error }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey;

    #[test]
    fn program_err_log() {
        let log = "Program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 failed: custom program error: 0x1771";
        let LoggedTransactionFailure { program, error } = check_for_program_error(log).unwrap();
        assert_eq!(
            pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"),
            program
        );
        assert_eq!("custom program error: 0x1771", error);
    }
}
