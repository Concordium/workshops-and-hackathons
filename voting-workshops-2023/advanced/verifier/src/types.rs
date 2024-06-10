use concordium_rust_sdk::{
    common::{Serial, Versioned},
    endpoints::{QueryError, RPCError},
    id::{
        constants::{ArCurve, AttributeKind},
        id_proof_types::{Proof, Statement},
        types::{AccountAddress, GlobalContext},
    },
    types::CredentialRegistrationID,
};
use ed25519_dalek::Keypair;
use serde_hex::{SerHex, Strict};
use std::sync::Arc;

/// Data needed for running the verifier server.
#[derive(Clone)]
pub struct Server {
    pub signing_keypair: Arc<Keypair>,
    pub global_context: Arc<GlobalContext<ArCurve>>,
}

/// An internal error type used by this server to manage error handling.
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error("Not allowed")]
    NotAllowed,
    #[error("Invalid proof")]
    InvalidProofs,
    #[error("Node access error: {0}")]
    NodeAccess(#[from] QueryError),
    #[error("Issue with credential.")]
    Credential,
    #[error("Statement not allowed.")]
    StatementNotAllowed,
}

impl From<RPCError> for ProofError {
    fn from(err: RPCError) -> Self {
        Self::NodeAccess(err.into())
    }
}

impl warp::reject::Reject for ProofError {}

#[derive(serde::Serialize)]
/// Response in case of an error. This is going to be encoded as a JSON body
/// with fields 'code' and 'message'.
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct ProofRequest {
    pub statement: Statement<ArCurve, AttributeKind>,
    pub address: AccountAddress,
    pub proof: ProofWithContext,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct ProofWithContext {
    pub credential: CredentialRegistrationID,
    pub proof: Versioned<Proof<ArCurve, AttributeKind>>,
}

/// The data used for the signature message to be signed and returned after verifying a proof.
pub struct SignatureMessageData {
    /// The account address for which the proof was verified.
    pub account_address: AccountAddress,
    /// The country code for the country which the account does *not* have residency in.
    pub country_code: String,
}

impl Serial for SignatureMessageData {
    fn serial<B: concordium_rust_sdk::common::Buffer>(&self, out: &mut B) {
        // Write the 32 bytes for the account address.
        self.account_address.serial(out);
        // Write the two bytes for the country code.
        out.write_all(self.country_code.as_bytes())
            .expect("Writing to buffer should never fail.");
    }
}

/// A wrapper around the bytes from [`ed25519_dalek::Signature`] which implements [`serde::Serialize`] by converting to hex.
#[derive(serde::Serialize)]
pub struct HexSignature(#[serde(with = "SerHex::<Strict>")] pub [u8; 64]);
