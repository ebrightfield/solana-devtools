use solana_sdk::bs58;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LocalnetConfigurationError>;
#[derive(Debug, Error)]
pub enum LocalnetConfigurationError {
    #[error("No output directory provided, cannot build JSON files.")]
    NoOutputDirectory,
    #[error("Could not find program .so binary {0}")]
    MissingProgramSoFile(String),
    #[error("Duplicate account pubkeys: {0:?}")]
    DuplicateAccountPubkey(Vec<String>),
    #[error("Duplicate account names: {0:?}")]
    DuplicateAccountName(Vec<String>),
    #[error("Duplicate program: {0:?}")]
    DuplicateProgramPubkey(String),
    #[error("Could not parse account JSON: {0}")]
    InvalidAccountJson(serde_json::Error),
    #[error("Could not parse base58 account data: {0}")]
    InvalidBase58AccountData(bs58::decode::Error),
    #[error("Could not parse base64 account data: {0}")]
    InvalidBase64AccountData(base64::DecodeError),
    #[error("Could not read/write to file: {0}: {1}")]
    FileReadWriteError(String, std::io::Error),
    #[error("Could not read/write to file: {0}: {1}")]
    SerdeFileReadWriteFailure(String, serde_json::Error),
    #[error("Could not deserialize account data: {0}")]
    AnchorAccountError(anchor_lang::error::Error),
    #[error("Could not fetch account data to clone: {0}")]
    ClonedAccountRpcError(solana_client::client_error::ClientError),
    #[error("Failed to parse IDL from lib.rs: {0}")]
    IdlParseError(String),
    #[error("Failed to serialize IDL to JSON bytes: {0}")]
    IdlSerializationError(String),
    #[error("Failed to create a BPF runtime environment: {0}")]
    EbpfError(String),
}
