mod handlers;
mod types;
use crate::handlers::*;
use crate::types::*;

use anyhow::Context;
use clap::Parser;
use concordium_rust_sdk::v2::BlockIdentifier;
use ed25519_dalek::{PublicKey, SecretKey};
use log::info;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use warp::Filter;

/// Structure used to receive the correct command line arguments.
#[derive(clap::Parser, Debug)]
#[clap(arg_required_else_help(true))]
#[clap(version, author)]
struct IdVerifierConfig {
    #[clap(
        long = "node",
        help = "GRPC V2 interface of the node.",
        default_value = "http://localhost:20000"
    )]
    endpoint: concordium_rust_sdk::v2::Endpoint,
    #[clap(
        long = "port",
        default_value = "8100",
        help = "Port on which the server will listen on."
    )]
    port: u16,
    #[structopt(
        long = "log-level",
        default_value = "debug",
        help = "Maximum log level."
    )]
    log_level: log::LevelFilter,
    #[structopt(
        long = "public-key",
        default_value = "public_key.bin",
        help = "Location of the public key in binary format."
    )]
    public_key: PathBuf,
    #[structopt(
        long = "secret-key",
        default_value = "secret_key.bin",
        help = "Location of the secret key in binary format."
    )]
    secret_key: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the command line parameters.
    let app = IdVerifierConfig::parse();
    let mut log_builder = env_logger::Builder::new();
    // Only log the current module (main).
    log_builder.filter_level(app.log_level);
    log_builder.init();

    // Set up a client for communicating with the node.
    let mut client = concordium_rust_sdk::v2::Client::new(app.endpoint).await?;
    // Retrieve the global context from the node.
    let global_context = client
        .get_cryptographic_parameters(BlockIdentifier::LastFinal)
        .await?
        .response;

    log::debug!("Acquired data from the node.");

    // Get the public and secret keys.
    let public_key = PublicKey::from_bytes(
        &fs::read(&app.public_key).context("Could not read public key file")?,
    )
    .context("Could not deserialize public key")?;
    let secret_key = SecretKey::from_bytes(
        &fs::read(&app.secret_key).context("Could not read secret key file")?,
    )
    .context("Could not deserialize secret key")?;

    // Create the server state.
    let state = Server {
        signing_keypair: Arc::new(ed25519_dalek::Keypair {
            secret: secret_key,
            public: public_key,
        }),
        global_context: Arc::new(global_context),
    };

    // Allow CORS.
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("Content-Type")
        .allow_method("POST");

    // Setup the handler for the the `/api/prove` endpoint.
    let provide_proof = warp::post()
        .and(warp::filters::body::content_length_limit(50 * 1024))
        .and(warp::path!("api" / "prove"))
        .and(warp::body::json())
        .and_then(move |request: ProofRequest| {
            info!("Got a ProofRequest: {:?}", request);
            handle_provide_proof(client.clone(), state.clone(), request)
        });

    info!("Starting up HTTP server. Listening on port {}.", app.port);

    // Run the server.
    let server = provide_proof
        .recover(handle_rejection)
        .with(cors)
        .with(warp::trace::request());
    warp::serve(server).run(([0, 0, 0, 0], app.port)).await;

    Ok(())
}
