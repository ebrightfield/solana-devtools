use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use anyhow::anyhow;
use solana_clap_v3_utils::keypair::keypair_from_seed_phrase;
use solana_program::pubkey::Pubkey;
use solana_remote_wallet::locator::Locator;
use solana_remote_wallet::remote_keypair::{generate_remote_keypair, RemoteKeypair};
use solana_remote_wallet::remote_wallet::maybe_wallet_manager;
use solana_sdk::derivation_path::DerivationPath;
use solana_sdk::signature::{Keypair, Presigner, PresignerError, read_keypair, read_keypair_file, Signature, SignerError};
use solana_sdk::signer::Signer;

// Keypair variant -- interactive, input seed phrase, takes a derivation path
const PROMPT_URI_PREFIX: &str = "prompt";
// Keypair variant -- interactive, input JSON string of a keypair file
const STDIN_URI_PREFIX: &str = "stdin";

// Keypair variant -- filepath to a keypair file
const FILE_URI_PREFIX: &str = "file";

// RemoteKeypair variant -- interactive, for communicating with hardware wallets
const USB_URI_PREFIX: &str = "usb";

// Presigner variant -- a pubkey and signature to a presigned transaction
const PRESIGN_URI_PREFIX: &str = "presigner://";

/// The same suite of input modes that are available with
/// the `solana-cli` crate, but returns a concrete type instead
/// of a trait object.
pub enum ConcreteSigner {
    /// `prompt://` and `file://` and `stdin://`
    Keypair(Keypair),
    /// `usb://`
    RemoteKeypair(RemoteKeypair),
    /// `presign://<pubkey>=<signature>`
    Presigner(Presigner),
}

impl Debug for ConcreteSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcreteSigner::Keypair(k) => write!(f, "{}", format!("ConcreteSigner::Keypair({:?})", k)),
            ConcreteSigner::RemoteKeypair(_) => write!(f, "ConcreteSigner::RemoteKeypair"),
            ConcreteSigner::Presigner(k) => write!(f, "{}", format!("{:?}", k)),
        }
    }
}

impl ConcreteSigner {
    pub fn new(source: &str) -> anyhow::Result<Self> {
        match uriparse::URIReference::try_from(source) {
            Err(e) => Err(anyhow!("Unrecognized source uri: {}", e)),
            Ok(uri) => {
                if let Some(scheme) = uri.scheme() {
                    let scheme = scheme.as_str().to_ascii_lowercase();
                    match scheme.as_str() {
                        PROMPT_URI_PREFIX => {
                            let d = DerivationPath::from_uri_any_query(&uri)?;
                            return Ok(ConcreteSigner::Keypair(keypair_from_seed_phrase(
                                "keypair",
                                false,
                                false,
                                d,
                                false,
                            ).map_err(|_| anyhow!("Failed to create keypair from seed phrase"))?));
                        },
                        FILE_URI_PREFIX => {
                            return Ok(ConcreteSigner::Keypair(
                                read_keypair_file(uri.path().to_string())
                            .map_err(|_| anyhow!("Couldn't find or parse keypair file"))?
                            ));
                        },
                        USB_URI_PREFIX => {
                            let d = DerivationPath::from_uri_any_query(&uri)?;
                            let locator = Locator::new_from_uri(&uri)?;
                            let wallet_manager = maybe_wallet_manager()?;
                            if let Some(wallet_manager) = wallet_manager {
                                return Ok(ConcreteSigner::RemoteKeypair(generate_remote_keypair(
                                    locator,
                                    d.unwrap_or_default(),
                                    &wallet_manager,
                                    true,
                                    "keypair",
                                )?));
                            } else {
                                return Err(anyhow!("No wallet manager could be created"));
                            }
                        },
                        STDIN_URI_PREFIX => {
                            let mut stdin = std::io::stdin();
                            return Ok(ConcreteSigner::Keypair(
                                read_keypair(&mut stdin)
                                    .map_err(|_| anyhow!("Unable to read keypair from stdin"))?));
                        },
                        PRESIGN_URI_PREFIX => {
                            return Ok(ConcreteSigner::Presigner(
                                try_presigner(&source)
                                    .map_err(|_| anyhow!("Unable to read presigner {}", &source))?));
                        },
                        unknown => { return Err(anyhow!("Unrecognized prefix: {}", unknown)); }
                    }
                } else {
                    return Ok(ConcreteSigner::Keypair(
                        read_keypair_file(uri.path().to_string())
                            .map_err(|_| anyhow!("Couldn't find or parse keypair file"))?
                    ));
                }
            }
        }
    }

}

impl From<Keypair> for ConcreteSigner {
    fn from(value: Keypair) -> Self {
        Self::Keypair(value)
    }
}

impl From<RemoteKeypair> for ConcreteSigner {
    fn from(value: RemoteKeypair) -> Self {
        Self::RemoteKeypair(value)
    }
}

impl Signer for ConcreteSigner {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        match &self {
            ConcreteSigner::Keypair(s) => s.try_pubkey(),
            ConcreteSigner::RemoteKeypair(s) => s.try_pubkey(),
            ConcreteSigner::Presigner(s) => s.try_pubkey(),
        }
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        match &self {
            ConcreteSigner::Keypair(s) => s.try_sign_message(message),
            ConcreteSigner::RemoteKeypair(s) => s.try_sign_message(message),
            ConcreteSigner::Presigner(s) => s.try_sign_message(message),
        }
    }

    fn is_interactive(&self) -> bool {
        match &self {
            ConcreteSigner::Keypair(s) => s.is_interactive(),
            ConcreteSigner::RemoteKeypair(s) => s.is_interactive(),
            ConcreteSigner::Presigner(s) => s.is_interactive(),
        }
    }
}

/// Expects pubkey and signature separated by an "=" sign. e.g. "abcd=7890"
pub fn try_presigner(value: &str) -> Result<Presigner, SignerError> {
    let mut signer = value.split('=');
    let pubkey = Pubkey::from_str(signer.next()
        .ok_or(SignerError::PresignerError(PresignerError::VerificationFailure))?
    ).map_err(|_| SignerError::PresignerError(PresignerError::VerificationFailure))?;
    let signature = Signature::from_str(signer.next()
        .ok_or(SignerError::PresignerError(PresignerError::VerificationFailure))?
    ).map_err(|_| SignerError::PresignerError(PresignerError::VerificationFailure))?;
    Ok(Presigner::new(&pubkey, &signature))
}
