use std::task::Poll;

use futures::future::{self, BoxFuture};
use solana_sdk::{
    offchain_message::OffchainMessage,
    sanitize::SanitizeError,
    signature::Signature,
    signer::Signer,
    transaction::{TransactionError, VersionedTransaction},
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct SignedPayload<T> {
    pub t: T,
    pub signature: Signature,
}

pub struct SignerService {
    signer: Box<dyn Signer>,
}

impl Service<VersionedTransaction> for SignerService {
    type Response = SignedPayload<VersionedTransaction>;

    type Error = TransactionError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: VersionedTransaction) -> Self::Future {
        let result = match req.verify_and_hash_message() {
            Ok(hash) => {
                let signature = self.signer.sign_message(hash.as_ref());
                Ok(SignedPayload { t: req, signature })
            }
            Err(e) => Err(e),
        };
        Box::pin(future::ready(result))
    }
}

impl Service<OffchainMessage> for SignerService {
    type Response = SignedPayload<OffchainMessage>;

    type Error = SanitizeError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: OffchainMessage) -> Self::Future {
        let result = match req.sign(&*self.signer) {
            Ok(signature) => Ok(SignedPayload { t: req, signature }),
            Err(e) => Err(e),
        };
        Box::pin(future::ready(result))
    }
}
