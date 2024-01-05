use crate::idl_sdk::account::serialize_and_compress_idl;
use anchor_lang::idl::{IdlAccount, IdlInstruction};
use anchor_lang::{system_program, AnchorSerialize};
use anchor_syn::idl::Idl;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use std::error::Error;

/// The recommended way to create an IDL account is through the Anchor CLI.
/// Create and resize the PDA which will become a program's primary IDL account.
/// Anchor does not CPI to the system program to create IDL accounts.
/// This is because IDL accounts are often >10kb, which is the max new data allocation
/// for a top-level instruction.
/// Therefore, IDL creation takes place across several top-level instructions,
/// one to create the account (this function creates that instruction), and one to
/// instantiate the account as an IDL buffer (see the function [create_buffer]).
/// The max length is 60kb.
pub fn idl_init_instructions(owner: Pubkey, program_id: Pubkey, data_len: u64) -> Vec<Instruction> {
    // TODO create_idl_account
    // TODO resize_idl_account n times based on data len
    todo!()
}

/// Get several `idl_write` instructions to successively write data to an IDL account.
pub fn idl_write_instructions(
    program_id: Pubkey,
    buffer: Pubkey,
    authority: Pubkey,
    idl: &Idl,
) -> Result<Vec<Instruction>, Box<dyn Error>> {
    // Remove the metadata
    let mut idl = idl.clone();
    idl.metadata = None;
    // Serialize and compress the idl.
    let idl_data = serialize_and_compress_idl(&idl)?;

    // Create instructions
    let mut instructions = vec![];
    const MAX_WRITE_SIZE: usize = 1000;
    let mut offset = 0;
    while offset < idl_data.len() {
        let start = offset;
        let end = std::cmp::min(offset + MAX_WRITE_SIZE, idl_data.len());
        instructions.push(idl_write(
            program_id,
            buffer,
            authority,
            idl_data[start..end].to_vec(),
        ));
        offset += MAX_WRITE_SIZE;
    }
    Ok(instructions)
}

/// Create a program's IDL account.
pub fn create_idl_account(program_id: Pubkey, authority: Pubkey, data_len: u64) -> Instruction {
    let program_signer = Pubkey::find_program_address(&[], &program_id).0;
    let idl_address = IdlAccount::address(&program_id);
    let accounts = vec![
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(idl_address, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(program_id, false),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::Create { data_len }.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Add up to 10kb more to an account. Resizing to a smaller account is currently not allowed.
pub fn resize_account(program_id: Pubkey, authority: Pubkey, data_len: u64) -> Instruction {
    let idl_address = IdlAccount::address(&program_id);
    let accounts = vec![
        AccountMeta::new(idl_address, false),
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::Resize { data_len }.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initialize an IDL buffer to perform an upgrade.
pub fn create_buffer(program_id: Pubkey, buffer: Pubkey, authority: Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(buffer, false),
        AccountMeta::new(authority, true),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::CreateBuffer.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Close the program's IDL account. This is the first step in the IDL upgrade process.
/// However, it cannot be composed with the rest of the IDL upgrade process. This is due to
/// how Anchor handles account close operations.
pub fn close_account(
    program_id: Pubkey,
    authority: Pubkey,
    sol_destination: Pubkey,
) -> Instruction {
    let idl_address = IdlAccount::address(&program_id);
    let accounts = vec![
        AccountMeta::new(idl_address, false),
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(sol_destination, false),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::Close.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// The "finishing" instruction of an IDL upgrade.
/// Copies the trailing data from a source IDL buffer to a target IDL buffer.
/// Does not directly modify the IDL account's header which stores the authority and data length.
/// The IDL authority passed must be the current authority of both
/// the source buffer and the target.
/// The target IDL account must be at least the size of the source buffer.
pub fn set_buffer(program_id: Pubkey, source_buffer: Pubkey, idl_authority: Pubkey) -> Instruction {
    let target_buffer = IdlAccount::address(&program_id);
    let accounts = vec![
        AccountMeta::new(source_buffer, false),
        AccountMeta::new(target_buffer, false),
        AccountMeta::new_readonly(idl_authority, true),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::SetBuffer.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Resize an IDL buffer account's size, without touching the `data_len` parameter on the IDL header.
/// It only adjusts the `data_len` of the underlying Solana account, and this instruction will fail
/// on any account that already has a non-zero `data_len` in the header.
/// Therefore, this instruction can only be used to resize brand-new buffer accounts.
/// Resizing existing IDL accounts is thus not possible.
/// Instead, one must first close the account, and then re-open a new one with a different size.
pub fn resize_buffer(
    program_id: Pubkey,
    buffer: Pubkey,
    authority: Pubkey,
    data_len: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(buffer, false),
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut IdlInstruction::Resize { data_len }.try_to_vec().unwrap());
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Append data to the IDL account or buffer,
/// and increment the IDL header's `data_len` field by the length of the data written.
pub fn idl_write(
    program_id: Pubkey,
    buffer: Pubkey,
    authority: Pubkey,
    idl_data: Vec<u8>,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(buffer, false),
        AccountMeta::new(authority, true),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(
        &mut IdlInstruction::Write { data: idl_data }
            .try_to_vec()
            .unwrap(),
    );
    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Transfer the IDL authority.
pub fn idl_set_authority(
    program_id: Pubkey,
    buffer: Pubkey,
    authority: Pubkey,
    new_authority: Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(buffer, false),
        AccountMeta::new(authority, true),
    ];
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(
        &mut IdlInstruction::SetAuthority { new_authority }
            .try_to_vec()
            .unwrap(),
    );
    Instruction {
        program_id,
        accounts,
        data,
    }
}
