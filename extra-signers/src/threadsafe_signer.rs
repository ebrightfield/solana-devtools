use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer, SignerError};
use std::sync::{Arc, Mutex};

/// Basic struct that imbues a [T: Signer] with [Clone + Send + Sync].
#[derive(Debug)]
pub struct ThreadsafeSigner<T: Signer> {
    pub inner: Arc<Mutex<T>>,
}

impl<T: Signer> ThreadsafeSigner<T> {
    #[allow(dead_code)]
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<T: Signer> Clone for ThreadsafeSigner<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Signer> Signer for ThreadsafeSigner<T> {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.inner.lock().unwrap().pubkey())
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.inner.lock().unwrap().try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        self.inner.lock().unwrap().is_interactive()
    }
}

#[cfg(test)]
mod tests {
    use crate::threadsafe_signer::ThreadsafeSigner;
    use solana_sdk::signature::keypair_from_seed;
    use solana_sdk::signature::Signer;
    use std::thread;

    #[test]
    fn threadsafe_keypair() {
        let keypair = keypair_from_seed(&[0u8; 32]).unwrap();
        let keypair = ThreadsafeSigner::new(keypair);
        let pubkey = keypair.pubkey();
        let data = [1u8];
        let sig = keypair.sign_message(&data);
        let takes_trait_object = |signer: Box<dyn Signer>| signer.pubkey();

        // Test thread safety
        {
            let keypair2 = keypair.clone();
            thread::spawn(move || {
                let data = [1u8];
                let sig = keypair2.sign_message(&data);
                // Signer
                assert_eq!(keypair2.try_pubkey().unwrap(), pubkey);
                assert_eq!(keypair2.pubkey(), pubkey);
                assert_eq!(keypair2.try_sign_message(&data).unwrap(), sig);
                assert_eq!(keypair2.sign_message(&data), sig);
                let _ = takes_trait_object(Box::new(keypair2));
            });
        }

        // Signer
        assert_eq!(keypair.try_pubkey().unwrap(), pubkey);
        assert_eq!(keypair.pubkey(), pubkey);
        assert_eq!(keypair.try_sign_message(&data).unwrap(), sig);
        assert_eq!(keypair.sign_message(&data), sig);
        let _ = takes_trait_object(Box::new(keypair));
    }
}
