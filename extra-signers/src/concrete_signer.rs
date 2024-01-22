use base64::{engine::general_purpose::STANDARD, Engine};
use bip39::{Language, Mnemonic, Seed};
use solana_program::pubkey::Pubkey;
#[cfg(feature = "remote-wallet")]
use solana_remote_wallet::{
    locator::Locator,
    remote_keypair::{generate_remote_keypair, RemoteKeypair},
    remote_wallet::maybe_wallet_manager
};
use solana_sdk::bs58;
use solana_sdk::derivation_path::DerivationPath;
use solana_sdk::signature::{
    read_keypair_file, Keypair, Presigner,
    PresignerError, Signature, SignerError,
};
use solana_sdk::signer::{SeedDerivable, Signer};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use rpassword::prompt_password;
use uriparse::URIReference;

// Keypair variant -- interactive, input seed phrase, takes a derivation path
const PROMPT_URI_PREFIX: &str = "prompt";
// Keypair variant -- interactive, input JSON string of a keypair file
const STDIN_URI_PREFIX: &str = "stdin";

// Keypair variant -- filepath to a keypair file
const FILE_URI_PREFIX: &str = "file";

// RemoteKeypair variant -- interactive, for communicating with hardware wallets
const USB_URI_PREFIX: &str = "usb";

// Presigner variant -- a pubkey and signature to a presigned transaction
const PRESIGN_URI_PREFIX: &str = "presigner";

/// The same suite of input modes that are available with
/// the `solana-cli` crate, but returns a concrete type instead
/// of a trait object.
pub enum ConcreteSigner {
    /// `prompt://` and `file://` and `stdin://`
    Keypair(Keypair, Option<DerivationPath>),
    /// `usb://`
    #[cfg(feature = "remote-wallet")]
    RemoteKeypair(RemoteKeypair),
    /// `presign://<pubkey>=<signature>`
    Presigner(Presigner),
}

impl ConcreteSigner {
    pub fn keypair(k: Keypair, derivation_path: Option<DerivationPath>) -> Self {
        Self::Keypair(k, derivation_path)
    }

    #[cfg(feature = "remote-wallet")]
    pub fn remote_keypair(k: RemoteKeypair) -> Self {
        Self::RemoteKeypair(k)
    }

    pub fn presigner(k: Presigner) -> Self {
        Self::Presigner(k)
    }

    pub fn from_seed_and_derivation_path<'a>(
        bytes: impl Into<&'a [u8]>,
        derivation_path: Option<DerivationPath>,
        legacy: bool,
    ) -> Result<Self, SignerError> {
        if legacy {
            Keypair::from_seed(bytes.into())
                .map(|k| ConcreteSigner::Keypair(k, None))
                .map_err(|e| {
                    SignerError::Custom(format!(
                        "failed to interpet seed phrase or derivation path: {}",
                        e
                    ))
                })
        } else {
            Keypair::from_seed_and_derivation_path(bytes.into(), derivation_path.clone())
                .map(|k| ConcreteSigner::Keypair(k, derivation_path))
                .map_err(|e| {
                    SignerError::Custom(format!(
                        "failed to interpet seed phrase or derivation path: {}",
                        e
                    ))
                })
        }
    }

    pub fn from_file(p: & impl AsRef<Path>) -> Result<Self, SignerError> {
        Ok(ConcreteSigner::Keypair(
            read_keypair_file(p).map_err(|e| {
                SignerError::Custom(format!("could not find or parse keypair file: {}", e))
            })?,
        None,
        ))
    }

    pub fn from_file_with_derivation_path(p: & impl AsRef<Path>, derivation_path: DerivationPath) -> Result<Self, SignerError> {
        let file = File::open(p.as_ref())
            .map_err(|e|
                SignerError::Custom(format!("could not find or open keypair file: {}", e))
            )?;
        let bytes: Vec<u8> = serde_json::from_reader(file)
            .map_err(|e|
                SignerError::Custom(format!("could not parse contents of keypair file: {}", e))
            )?;
        Self::from_seed_and_derivation_path(bytes.as_slice(), Some(derivation_path), false)
    }

    pub fn from_raw_secret(
        secret: &str,
        derivation_path: Option<DerivationPath>,
        legacy: bool,
    ) -> Result<Self, SignerError> {
        #[cfg(feature = "serde_json")]
        if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(secret) {
            return Self::from_seed_and_derivation_path(bytes.as_slice(), derivation_path, legacy);
        }
        if let Ok(bytes) = bs58::decode(secret).into_vec() {
            return Self::from_seed_and_derivation_path(bytes.as_slice(), derivation_path, legacy);
        }
        #[cfg(feature = "base64")]
        if let Ok(bytes) = STANDARD.decode(secret) {
            return Self::from_seed_and_derivation_path(bytes.as_slice(), derivation_path, legacy);
        }
        let mut error_message = format!("failed to interpret seed phrase as Base58 bytes");
        #[cfg(feature = "base64")]
        error_message.extend(" or Base64 bytes".chars());
        #[cfg(feature = "serde_json")]
        error_message.extend(" or a JSON string of a byte array".chars());
        Err(SignerError::Custom(error_message))
    }

    pub fn from_seed_phrase_and_derivation_path(
        seed_phrase: &str,
        derivation_path: Option<DerivationPath>,
        passphrase: &str,
        legacy: bool,
    ) -> Result<Self, SignerError> {
        let sanitized = seed_phrase
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");
        let mnemonic = || {
            for language in &[
                Language::English,
                Language::ChineseSimplified,
                Language::ChineseTraditional,
                Language::Japanese,
                Language::Spanish,
                Language::Korean,
                Language::French,
                Language::Italian,
            ] {
                if let Ok(mnemonic) = Mnemonic::from_phrase(&sanitized, *language) {
                    return Ok(mnemonic);
                }
            }
            Err(SignerError::Custom(format!(
                "Can't get mnemonic from seed phrase"
            )))
        };
        let mnemonic = mnemonic()?;
        let seed = Seed::new(&mnemonic, &passphrase);
        Self::from_seed_and_derivation_path(
            seed.as_bytes(),
            derivation_path,
            legacy,
        )
    }

    pub fn derivation_path(&self) -> Option<&DerivationPath> {
        match &self {
            ConcreteSigner::Keypair(_, d) => {
                d.as_ref()
            }
            #[cfg(feature = "remote-wallet")]
            ConcreteSigner::RemoteKeypair(k) => {
                Some(&k.derivation_path)
            }
            ConcreteSigner::Presigner(_) => {
                None
            }
        }
    }
}

impl<'a> TryFrom<URIReference<'a>> for ConcreteSigner {
    type Error = SignerError;

    fn try_from(uri: URIReference) -> Result<Self, Self::Error> {
        let d = DerivationPath::from_uri_any_query(&uri).map_err(|e| {
            SignerError::Custom(format!("failed to interpret derivation path: {}", e))
        })?;
        if let Some(scheme) = uri.scheme() {
            let scheme = scheme.as_str().to_ascii_lowercase();
            return match scheme.as_str() {
                PROMPT_URI_PREFIX => {
                    let secret = prompt_password(format!("keypair secret: "))
                        .map_err(|e|
                            SignerError::Custom(format!("Unable to read from stdin: {}", e))
                        )?;
                    if let Ok(this) = Self::from_raw_secret(&secret, d.clone(), false) {
                        Ok(this)
                    } else {
                        Self::from_seed_phrase_and_derivation_path(
                            &secret, d, "", false
                        )
                    }
                }
                FILE_URI_PREFIX => Self::from_file(&uri.path().to_string()),
                #[cfg(feature = "remote-wallet")]
                USB_URI_PREFIX => {
                    let locator = Locator::new_from_uri(&uri).map_err(|e| {
                        SignerError::Custom(format!("remote wallet locator error: {}", e))
                    })?;
                    let wallet_manager = maybe_wallet_manager()
                        .map_err(|e| Into::<SignerError>::into(e))?
                        .ok_or(SignerError::Custom(format!(
                            "no remote wallet manager available"
                        )))?;
                    let d = d.unwrap_or_default();
                    let k = generate_remote_keypair(
                        locator,
                        d.clone(),
                        &wallet_manager,
                        true,
                        "keypair",
                    )
                        .map_err(|e| SignerError::from(e))?;
                    Ok(Self::remote_keypair(k))
                }
                STDIN_URI_PREFIX => {
                    let mut stdin = std::io::stdin();
                    let mut buffer = String::new();
                    stdin.read_to_string(&mut buffer)
                        .map_err(|e|
                            SignerError::Custom(format!("Unable to read from stdin: {}", e))
                        )?;
                    if let Ok(this) = Self::from_raw_secret(&buffer, d.clone(), false) {
                        Ok(this)
                    } else {
                        Self::from_seed_phrase_and_derivation_path(
                            &buffer, d, "", false
                        )
                    }
                }
                PRESIGN_URI_PREFIX => {
                    Ok(ConcreteSigner::Presigner(try_presigner(&uri.to_string())?))
                }
                unknown => Err(SignerError::Custom(format!(
                    "Unrecognized prefix: {}",
                    unknown
                ))),
            };
        } else {
            Self::from_file(&uri.path().to_string())
        }
    }
}

impl Debug for ConcreteSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcreteSigner::Keypair(k, d) => {
                if let Some(d) = d {
                    write!(f, "ConcreteSigner::Keypair({}, {:?})", k.pubkey(), d)
                } else {
                    write!(f, "ConcreteSigner::Keypair({})", k.pubkey())
                }
            }
            #[cfg(feature = "remote-wallet")]
            ConcreteSigner::RemoteKeypair(k) => {
                write!(f, "ConcreteSigner::RemoteKeypair({}, {:?})", k.pubkey(), k.derivation_path)
            }
            ConcreteSigner::Presigner(k) => write!(f, "ConcreteSigner::Presigner({})", k.pubkey()),
        }
    }
}

impl FromStr for ConcreteSigner {
    type Err = SignerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let reference = URIReference::try_from(s).map_err(|e| {
            SignerError::Custom(format!("could not find or parse keypair file: {}", e))
        })?;
        Self::try_from(reference)
    }
}

impl TryFrom<String> for ConcreteSigner {
    type Error = SignerError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let reference = URIReference::try_from(s.as_str()).map_err(|e| {
            SignerError::Custom(format!("could not find or parse keypair file: {}", e))
        })?;
        Self::try_from(reference)
    }
}

impl From<Keypair> for ConcreteSigner {
    fn from(value: Keypair) -> Self {
        Self::Keypair(value, None)
    }
}

#[cfg(feature = "remote-wallet")]
impl From<RemoteKeypair> for ConcreteSigner {
    fn from(value: RemoteKeypair) -> Self {
        Self::RemoteKeypair(value)
    }
}

impl Signer for ConcreteSigner {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        match &self {
            ConcreteSigner::Keypair(k, _) => k.try_pubkey(),
            #[cfg(feature = "remote-wallet")]
            ConcreteSigner::RemoteKeypair(k) => k.try_pubkey(),
            ConcreteSigner::Presigner(s) => s.try_pubkey(),
        }
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        match &self {
            ConcreteSigner::Keypair(k, _) => k.try_sign_message(message),
            #[cfg(feature = "remote-wallet")]
            ConcreteSigner::RemoteKeypair(k) => k.try_sign_message(message),
            ConcreteSigner::Presigner(s) => s.try_sign_message(message),
        }
    }

    fn is_interactive(&self) -> bool {
        match &self {
            ConcreteSigner::Keypair(k, _) => k.is_interactive(),
            #[cfg(feature = "remote-wallet")]
            ConcreteSigner::RemoteKeypair(k) => k.is_interactive(),
            ConcreteSigner::Presigner(s) => s.is_interactive(),
        }
    }
}

/// Expects pubkey and signature separated by an "=" sign. e.g. "abcd=7890"
pub fn try_presigner(value: &str) -> Result<Presigner, SignerError> {
    let mut signer = value.split('=');
    let pubkey = Pubkey::from_str(signer.next().ok_or(SignerError::PresignerError(
        PresignerError::VerificationFailure,
    ))?)
    .map_err(|_| SignerError::PresignerError(PresignerError::VerificationFailure))?;
    let signature = Signature::from_str(signer.next().ok_or(SignerError::PresignerError(
        PresignerError::VerificationFailure,
    ))?)
    .map_err(|_| SignerError::PresignerError(PresignerError::VerificationFailure))?;
    Ok(Presigner::new(&pubkey, &signature))
}
