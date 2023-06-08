use crate::types::*;
use concordium_rust_sdk::{
    common::to_bytes,
    id::{
        id_proof_types::{AtomicStatement, AttributeNotInSetStatement},
        types::{AccountCredentialWithoutProofs, AttributeTag},
    },
    v2::BlockIdentifier,
};
use ed25519_dalek::Signer;
use log::warn;
use std::convert::Infallible;
use warp::{http::StatusCode, Rejection};

/// Handle the proof endpoint.
pub async fn handle_provide_proof(
    client: concordium_rust_sdk::v2::Client,
    state: Server,
    request: ProofRequest,
) -> Result<impl warp::Reply, Rejection> {
    let client = client.clone();
    let state = state.clone();
    match check_proof_worker(client, state, request).await {
        Ok(r) => Ok(warp::reply::json(&r)),
        Err(e) => {
            warn!("Request is invalid {:#?}.", e);
            Err(warp::reject::custom(e))
        }
    }
}

/// Handle causes of rejection by returning a human readable message and an error code.
pub async fn handle_rejection(err: Rejection) -> Result<impl warp::Reply, Infallible> {
    if err.is_not_found() {
        let code = StatusCode::NOT_FOUND;
        let message = "Not found.";
        Ok(mk_reply(message.into(), code))
    } else if let Some(ProofError::NotAllowed) = err.find() {
        let code = StatusCode::BAD_REQUEST;
        let message = "Needs proof.";
        Ok(mk_reply(message.into(), code))
    } else if let Some(ProofError::InvalidProofs) = err.find() {
        let code = StatusCode::BAD_REQUEST;
        let message = "Invalid proofs.";
        Ok(mk_reply(message.into(), code))
    } else if let Some(ProofError::StatementNotAllowed) = err.find() {
        let code = StatusCode::BAD_REQUEST;
        let message = "Statement not allowed.";
        Ok(mk_reply(message.into(), code))
    } else if let Some(ProofError::NodeAccess(e)) = err.find() {
        let code = StatusCode::INTERNAL_SERVER_ERROR;
        let message = format!("Cannot access the node: {}", e);
        Ok(mk_reply(message, code))
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        let code = StatusCode::BAD_REQUEST;
        let message = "Malformed body.";
        Ok(mk_reply(message.into(), code))
    } else {
        let code = StatusCode::INTERNAL_SERVER_ERROR;
        let message = "Internal error.";
        Ok(mk_reply(message.into(), code))
    }
}

/// Helper function to make the reply.
fn mk_reply(message: String, code: StatusCode) -> impl warp::Reply {
    let msg = ErrorResponse {
        message,
        code: code.as_u16(),
    };
    warp::reply::with_status(warp::reply::json(&msg), code)
}

/// Checks that the statement is valid and that the proof is correct.
async fn check_proof_worker(
    mut client: concordium_rust_sdk::v2::Client,
    state: Server,
    request: ProofRequest,
) -> Result<HexSignature, ProofError> {
    let cred_id = request.proof.credential;
    let acc_info = client
        .get_account_info(&request.address.into(), BlockIdentifier::LastFinal)
        .await?;

    // TODO The account may have more that one credential, check the remaining ones.
    let credential = acc_info
        .response
        .account_credentials
        .get(&0.into())
        .ok_or(ProofError::Credential)?;

    if to_bytes(credential.value.cred_id()) != to_bytes(&cred_id) {
        return Err(ProofError::Credential);
    }

    // Get the commitments from the credential.
    let commitments = match &credential.value {
        AccountCredentialWithoutProofs::Initial { icdv: _, .. } => {
            return Err(ProofError::NotAllowed);
        }
        AccountCredentialWithoutProofs::Normal { commitments, .. } => commitments,
    };

    // Check that the statement sent is that the account is *not* from one particular country.
    const COUNTRY_OF_RESIDENCY: u8 = 4;
    let country_code = match &request.statement.statements[..] {
        [AtomicStatement::AttributeNotInSet {
            statement:
                AttributeNotInSetStatement {
                    attribute_tag: AttributeTag(tag),
                    set,
                    ..
                },

        }]
            // The proof is about country of residency.
            if *tag == COUNTRY_OF_RESIDENCY
            // There is only one country listed
            && set.len() == 1
            // The country code is two bytes long
            && set.first().unwrap().0.bytes().len() == 2 =>
        {
            set.first().unwrap().0.clone()
        }
        _ => return Err(ProofError::StatementNotAllowed),
    };

    // The challenge is not really used here, as there is no temporal aspect to the proof,
    // but the challenge must match the one specified in the dapp.
    // Otherwise the proof won't be valid.
    let challenge = [0u8; 4];

    // Verify the proof
    if request.statement.verify(
        &challenge,
        &state.global_context,
        cred_id.as_ref(),
        commitments,
        &request.proof.proof.value,
    ) {
        // Construct the data to sign, which is the account address and country code.
        let message_data = SignatureMessageData {
            account_address: request.address,
            country_code,
        };
        let message = to_bytes(&message_data);
        // Sign the message.
        let signature = state.signing_keypair.sign(&message);
        // Use the wrapper `HexSignature` to make sure it is serialized as hex.
        let hex_signature = HexSignature(signature.into());
        // Return the signature as hex.
        Ok(hex_signature)
    } else {
        // Return an error if the proof is invalid.
        Err(ProofError::InvalidProofs)
    }
}
