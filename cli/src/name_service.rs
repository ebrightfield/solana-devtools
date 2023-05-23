use clap::{ArgMatches, Parser};
use anyhow::{anyhow, Result};
use base58::{FromBase58, ToBase58};
use solana_clap_v3_utils::keypair::{signer_from_path};
use solana_client::rpc_client::RpcClient;
use solana_sdk::hash::hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use spl_name_service::instruction::NameRegistryInstruction;
use spl_name_service::state::{NameRecordHeader, HASH_PREFIX, get_seeds_and_key};
use solana_devtools_cli_config::KeypairArg;


#[derive(Debug, Parser)]
pub enum NameServiceSubcommand {
    /// Create a new name record.
    Create {
        /// The plain-text name of the account
        name: String,
        /// Defaults to the configured `-k/--keypair` signer.
        /// Not required to sign.
        #[clap(long, parse(try_from_str=Pubkey::try_from))]
        owner: Option<Pubkey>,
        /// Optional parent name address. The owner will be fetched from on-chain.
        #[clap(long, parse(try_from_str=Pubkey::try_from))]
        parent: Option<Pubkey>,
        #[clap(long)]
        /// Optional class, passed as a signer path.
        class: Option<String>,
        /// Don't update the name account with name data
        #[clap(long)]
        no_update: bool,
        /// Optional amount of space, defaults to the byte length of the name encoded as UTF-8,
        /// which is the name account's minimum size unless the "no-update" flag is used.
        #[clap(long)]
        space: Option<usize>,
    },
    /// Read the state of a name account.
    Read {
        /// Pubkey of the name account.
        #[clap(parse(try_from_str=Pubkey::try_from))]
        name_account: Pubkey,
        #[clap(long)]
        /// Whether to interpret the data as Base58. Interpreted as UTF-8 otherwise.
        base58: bool,
    },
    /// Derive the PDA address given a name, and optionally a class and parent.
    DeriveAccount {
        /// The name seed
        name: String,
        /// Optional class name address
        #[clap(parse(try_from_str=Pubkey::try_from))]
        class: Option<Pubkey>,
        /// Optional parent name address
        #[clap(parse(try_from_str=Pubkey::try_from))]
        parent: Option<Pubkey>,
    },
    /// Update the data stored on a name record.
    Update {
        /// Pubkey of the name account.
        #[clap(parse(try_from_str=Pubkey::try_from))]
        name_account: Pubkey,
        /// Data to write to the name account.
        data: String,
        /// The byte offset to overwrite
        #[clap(long)]
        offset: u32,
        #[clap(long)]
        /// Whether to interpret the data as Base58. Interpreted as UTF-8 otherwise.
        base58: bool,
        #[clap(long)]
        /// If the `-k/--keypair` signer owns the parent of "name_account", you must
        /// supply the parent name account's address
        #[clap(parse(try_from_str=Pubkey::try_from))]
        parent_name_account: Option<Pubkey>,
    },
    /// Transfer ownership of a name account.
    Transfer {
        /// Pubkey of the name account.
        #[clap(parse(try_from_str=Pubkey::try_from))]
        name_account: Pubkey,
        /// Pubkey of the new owner of the name account.
        #[clap(parse(try_from_str=Pubkey::try_from))]
        new_owner: Pubkey,
        #[clap(long)]
        /// Optional class, passed as a signer path.
        class: Option<String>,
    },
    /// Delete a name account
    Delete {
        /// Pubkey of the name account.
        #[clap(parse(try_from_str=Pubkey::try_from))]
        name_account: Pubkey,
        /// Refund rent lamports to this account. Defaults to the `-k/--keypair` signer.
        #[clap(long, parse(try_from_str=Pubkey::try_from))]
        refund: Option<Pubkey>,
    },
    // TODO Realloc by deletion and re-creation
}

impl NameServiceSubcommand {
    pub fn process(
        self,
        client: &RpcClient,
        keypair: &KeypairArg,
        matches: &ArgMatches,
    ) -> Result<()> {
        match self {
            NameServiceSubcommand::Create {
                name, owner, parent,
                class, no_update, space,
            } => {
                let signer = keypair.resolve(&matches)?;
                let signer_pubkey = signer.pubkey();
                let class = if let Some(path) = class {
                    Some(signer_from_path(
                        &matches,
                        &path,
                        "keypair",
                        &mut None,
                    ).map_err(|_| anyhow!("Invalid signer path: {}", path))?)
                } else {
                    None
                };
                let class_pubkey = if let Some(class) = &class {
                    Some(class.pubkey())
                } else {
                    None
                };
                let owner = owner.unwrap_or(signer.pubkey());
                let parent_and_owner = if let Some(parent) = parent {
                    let data = client.get_account_data(&parent)?;
                    let name_record_header = NameRecordHeader::unpack_from_slice(
                        &data
                    )?;
                    Some((parent, name_record_header.owner))
                } else {
                    None
                };
                let lamports = client.get_minimum_balance_for_rent_exemption(
                    128 + NameRecordHeader::LEN + space.unwrap_or(name.as_bytes().len())
                )?;
                println!("Lamports: {}", lamports);
                let ix = create_name_instruction(
                    &name,
                    &signer_pubkey,
                    &owner,
                    parent_and_owner,
                    class_pubkey,
                    lamports,
                    space,
                )?;
                let hashed_name = hashed_name(&name);
                let (name_address, _) = get_seeds_and_key(
                    &SPL_NAME_SERVICE,
                    hashed_name.to_vec(),
                    class_pubkey.as_ref(),
                    parent.as_ref(),
                );
                println!("Creating name address {} with name data {}", &name_address, &name);
                let signers = if let Some(class) = class {
                    vec![signer, class]
                } else {
                    vec![signer]
                };
                let instructions = if !no_update {
                    let ix2 = update_name_instruction(
                        name_address,
                        signer_pubkey,
                        0,
                        name.into_bytes(),
                        parent,
                    )?;
                    vec![ix, ix2]
                } else {
                    vec![ix]
                };
                let tx = Transaction::new_signed_with_payer(
                    &instructions,
                    Some(&signer_pubkey),
                    &signers,
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            },
            NameServiceSubcommand::Read { name_account, base58 } => {
                let data = client.get_account_data(&name_account)?;
                let (header, remaining) = data.split_at(NameRecordHeader::LEN);
                let name_record_header = NameRecordHeader::unpack_from_slice(
                    &header
                )?;
                let remaining = if base58 {
                    remaining.to_base58()
                } else {
                    String::from_utf8(remaining.to_vec())
                        .map_err(|_| anyhow!("Could not deserialize name entry as UTF-8"))?
                };
                println!("Address: {}", name_account);
                println!("Owner: {}", name_record_header.owner);
                println!("Class: {}", name_record_header.class);
                println!("Parent Name: {}", name_record_header.parent_name);
                println!("Entry: {}", remaining);
            },
            NameServiceSubcommand::DeriveAccount { name, class, parent } => {
                let hashed_name = hashed_name(&name);
                let (name_address, _) = get_seeds_and_key(
                    &SPL_NAME_SERVICE,
                    hashed_name.to_vec(),
                    class.as_ref(),
                    parent.as_ref(),
                );
                println!("{}", name_address);
            },
            NameServiceSubcommand::Update { name_account, data, offset, base58, parent_name_account } => {
                let signer = keypair.resolve(&matches)?;
                let signer_pubkey = signer.pubkey();
                let data = if base58 {
                    data.from_base58().map_err(|_| anyhow!("Invalid base58 data"))?
                } else {
                    data.as_bytes().to_vec()
                };
                let ix = update_name_instruction(
                    name_account,
                    signer_pubkey,
                    offset,
                    data,
                    parent_name_account,
                )?;
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer_pubkey),
                    &vec![signer],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            }
            NameServiceSubcommand::Transfer { name_account, new_owner, class } => {
                let signer = keypair.resolve(&matches)?;
                let signer_pubkey = signer.pubkey();
                let class = if let Some(path) = class {
                    Some(signer_from_path(
                        &matches,
                        &path,
                        "keypair",
                        &mut None,
                    ).map_err(|_| anyhow!("Invalid signer path: {}", path))?)
                } else {
                    None
                };
                let class_pubkey = if let Some(class) = &class {
                    Some(class.pubkey())
                } else {
                    None
                };
                let signers = if let Some(class) = class {
                    vec![signer, class]
                } else {
                    vec![signer]
                };
                let ix = transfer_name_instruction(
                    &name_account,
                    &new_owner,
                    &signer_pubkey,
                    class_pubkey,
                )?;
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer_pubkey),
                    &signers,
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            }
            NameServiceSubcommand::Delete { name_account, refund } => {
                let signer = keypair.resolve(&matches)?;
                let signer_pubkey = signer.pubkey();
                let refund_account = refund.unwrap_or(signer_pubkey.clone());
                let ix = delete_name_instruction(
                    name_account,
                    signer_pubkey,
                    refund_account,
                )?;
                let tx = Transaction::new_signed_with_payer(
                    &[ix],
                    Some(&signer_pubkey),
                    &vec![signer],
                    client.get_latest_blockhash()?
                );
                let signature = client.send_transaction(&tx)
                    .map_err(|e| {
                        println!("{:#?}", &e);
                        e
                    })?;
                println!("{}", signature);
            }
        }
        Ok(())
    }
}

/// Both Devnet and Mainnet
pub const SPL_NAME_SERVICE: Pubkey = pubkey!("namesLPneVptA9Z5rqUDD9tMTWEJwofgaYwp8cawRkX");

pub fn hashed_name(name: &str) -> [u8; 32] {
    hash(format!("{}{}", HASH_PREFIX, name).as_bytes()).to_bytes()
}

/// `parent_and_owner` is a tuple of Parent name and the owner of the parent
pub fn create_name_instruction(
    name: &str,
    payer: &Pubkey,
    owner: &Pubkey,
    parent_and_owner: Option<(Pubkey, Pubkey)>,
    class: Option<Pubkey>,
    lamports: u64,
    space: Option<usize>,
) -> Result<Instruction> {
    let hashed_name = hashed_name(name);
    let additional_space = name.as_bytes().len();
    let space = space.unwrap_or(NameRecordHeader::LEN + additional_space) as u32;
    let data = NameRegistryInstruction::Create {
        hashed_name: hashed_name.to_vec(),
        lamports,
        space,
    };
    let (parent, parent_owner) = parent_and_owner
        .map(|(a, b)| (Some(a), Some(b)))
        .unwrap_or((None, None));
    let (name_address, _) = get_seeds_and_key(
        &SPL_NAME_SERVICE,
        hashed_name.to_vec(),
        class.as_ref(),
        parent.as_ref(),
    );
    Ok(spl_name_service::instruction::create(
        SPL_NAME_SERVICE,
        data,
        name_address,
        payer.clone(),
        owner.clone(),
        class,
        parent,
        parent_owner,
    )?)
}

pub fn update_name_instruction(
    name_account: Pubkey,
    signer: Pubkey,
    offset: u32,
    data: Vec<u8>,
    parent_name_account: Option<Pubkey>,
) -> Result<Instruction> {
    Ok(spl_name_service::instruction::update(
        SPL_NAME_SERVICE,
        offset,
        data,
        name_account,
        signer,
        parent_name_account,
    )?)
}

pub fn transfer_name_instruction(
    name_account: &Pubkey,
    new_owner: &Pubkey,
    signer: &Pubkey,
    class: Option<Pubkey>,
) -> Result<Instruction> {
    Ok(spl_name_service::instruction::transfer(
        SPL_NAME_SERVICE,
        new_owner.clone(),
        name_account.clone(),
        signer.clone(),
        class,
    )?)
}

pub fn delete_name_instruction(
    name_account: Pubkey,
    signer: Pubkey,
    refund_account: Pubkey,
) -> Result<Instruction> {
    Ok(spl_name_service::instruction::delete(
        SPL_NAME_SERVICE,
        name_account,
        signer,
        refund_account,
    )?)
}

// pub fn realloc_name_instruction(
//     payer: &Pubkey,
//     name_account: &Pubkey,
//     name_owner: &Pubkey,
//     space: u32,
// ) -> Result<Instruction> {
//     Ok(spl_name_service::instruction::realloc)
// }