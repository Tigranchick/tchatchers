//! This crate's purpose is to have a usable and functional CLI to perform simple operations or setup the tchatchers app.
//!
//! It can be handy since the implementations on the front might take some time, besides of helping the person trying
//! to install this application to have it running easily.

/// The actions that will be ran from this CLI usage.
mod actions;
/// The arguments of the CLI.
mod args;
/// The shared struct between the action and arg modules.
mod common;
/// The errors thrown during the runtime.
mod errors;

use std::process::{ExitCode, Termination};

use actions::{env::EnvAction, message::MessageAction, room::RoomAction};
use args::{message::MessageArgAction, CliArgs};
use clap::Parser;
use errors::CliError;

use crate::actions::user::UserAction;

#[macro_use]
extern crate derive_more;
use log::{debug, info};

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();

    match run_main().await {
        Ok(()) => {
            println!("The process ended with success.");
            ExitCode::SUCCESS
        }
        Err(err) => err.report(),
    }
}

/// Run the main logic and returns an exit code accordingly.
async fn run_main() -> Result<(), CliError> {
    let args: CliArgs = CliArgs::parse();
    debug!("Parsed CLI args: {:?}", args);

    match args.env {
        None => dotenv::dotenv().ok(),
        Some(v) => dotenv::from_filename(v).ok(),
    };

    match args.entity {
        args::CliEntityArg::User { action } => match action {
            args::user::UserArgAction::Create => {
                info!("Creating new user...");
                UserAction::create_user().await?
            }
            args::user::UserArgAction::Deactivate { user_identifier } => {
                info!("Deactivating user with identifier {}...", user_identifier);
                UserAction::update_activation_status(user_identifier, false).await?
            }
            args::user::UserArgAction::Activate { user_identifier } => {
                info!("Activating user with identifier {}...", user_identifier);
                UserAction::update_activation_status(user_identifier, true).await?
            }
            args::user::UserArgAction::Delete { user_identifier } => {
                info!("Deleting user with identifier {}...", user_identifier);
                UserAction::delete_user(user_identifier).await?
            }
            args::user::UserArgAction::Search { user_search } => {
                info!("Searching for user with search term {}...", user_search);
                UserAction::search_user(user_search).await?
            }
        },
        args::CliEntityArg::Message { action } => match action {
            MessageArgAction::Delete { messages_uuid } => {
                info!("Deleting messages with UUIDs: {:?}", messages_uuid);
                MessageAction::delete_messages(messages_uuid).await?
            }
        },
        args::CliEntityArg::Env { action } => match action {
            args::env::EnvArgAction::Create => {
                info!("Creating environment variables...");
                EnvAction::create()?
            }
            args::env::EnvArgAction::Check => {
                info!("Checking environment variables...");
                EnvAction::check_setup().await?
            }
            args::env::EnvArgAction::BuildNginxConf { output_file } => {
                info!("Building the nginx config file...");
                EnvAction::build_nginx_conf(&output_file)?
            }
        },
        args::CliEntityArg::Room { action } => match action {
            args::room::RoomArgAction::Clean { room_name } => {
                info!("Cleaning room {}...", room_name);
                RoomAction::delete_messages(&room_name).await?
            }
            args::room::RoomArgAction::GetMessages { room_name } => {
                info!("Getting the messages from room {}...", room_name);
                RoomAction::get_messages(&room_name).await?
            }
            args::room::RoomArgAction::Activity => {
                info!("Getting the activity...");
                RoomAction::get_activity().await?
            }
        },
    }
    Ok(())
}
