use std::{
    env::var,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    process::{Command, Output},
};

use askama::Template;
use dialoguer::{Confirm, Input, Password};
use which::which;

use crate::errors::CliError;

#[derive(Template, Debug, Default)]
#[template(path = "nginx.conf", ext = "txt", escape = "none")]
struct NginxConfigTemplate {
    http_only: bool,
    disable_security: bool,
    disable_http2: bool,
    version: String,
    server_name: String,
}

#[derive(Template, Debug, Default)]
#[template(path = "env", ext = "txt", escape = "none")]
struct EnvTemplate {
    postgres_host: String,
    postgres_port: u32,
    postgres_db_name: String,
    postgres_user_name: String,
    postgres_password: String,
    redis_host: String,
    redis_port: String,
    jwt_secret: String,
    refresh_token_secret: String,
    ssl_certificate_key: Option<String>,
    ssl_certificate_path: Option<String>,
    ssl_dhparam_path: Option<String>,
}

/// This struct provides functionality to interact with environment variables.
pub struct EnvAction;

const FILE_NAME: &str = ".env";

const CHECKMARK_EMOJI: &str = "\u{2714}";
const ERROR_EMOJI: &str = "\u{0058}";
const WARNING_EMOJI: &str = "\u{26A0}";

/// A constant array of tuples representing the environment variables that should be checked, along with their error types.
const ENV_VARS_TO_CHECK: [(&str, EnvironmentCheckErrorTypes); 13] = [
    ("DATABASE_URL", EnvironmentCheckErrorTypes::Warning),
    ("POSTGRES_DB", EnvironmentCheckErrorTypes::Error),
    ("POSTGRES_USER", EnvironmentCheckErrorTypes::Error),
    ("POSTGRES_PASSWORD", EnvironmentCheckErrorTypes::Error),
    ("REDIS_HOST", EnvironmentCheckErrorTypes::Error),
    ("POSTGRES_PORT", EnvironmentCheckErrorTypes::Warning),
    ("JWT_SECRET", EnvironmentCheckErrorTypes::Error),
    ("REFRESH_TOKEN_SECRET", EnvironmentCheckErrorTypes::Error),
    ("UID", EnvironmentCheckErrorTypes::Warning),
    ("GID", EnvironmentCheckErrorTypes::Warning),
    ("SSL_CERTIFICATE_PATH", EnvironmentCheckErrorTypes::Warning),
    ("SSL_CERTIFICATE_KEY", EnvironmentCheckErrorTypes::Warning),
    ("SSL_DHPARAM_PATH", EnvironmentCheckErrorTypes::Warning),
];

/// A constant array of program names to check if they exist in the PATH.
const PATH_PGM_TO_CHECK: [&str; 6] = [
    "docker",
    "cargo",
    "docker-compose",
    "npx",
    "trunk",
    "rustup",
];

/// A constant array of targets to check if they are installed for cargo.
const TARGETS_TO_CHECK: [&str; 1] = ["wasm32-unknown-unknown"];

/// An enum representing the types of errors for environment variable checks.
#[derive(Debug, PartialEq, Eq)]
enum EnvironmentCheckErrorTypes {
    Error,
    Warning,
}

impl EnvAction {
    /// Create a new `.env` file and populate it with database-related environment variables.
    pub fn create() -> Result<(), CliError> {
        if fs::read(FILE_NAME).is_ok() {
            let confirm_override = Confirm::new()
                .with_prompt("The .env file already exists, if you confirm the following dialog, any modification to the default values of the dialoguer will change the current environment file.")
                .default(false)
                .interact()?;
            if !confirm_override {
                return Ok(());
            } else {
                println!("The showed default values will be purposed from the existing .env file, skip changing them by pressing enter (default value).");
            }
        } else {
            println!("Setting up a new .env file");
        }

        let postgres_host: String = Input::new()
            .with_prompt("* Database host\nBe cautious, if you plan to deploy this in production, do not pick default localhost.")
            .default(std::env::var("POSTGRES_HOST").unwrap_or("localhost".into()))
            .interact_text()?;
        let postgres_port: u32 = Input::new()
            .with_prompt("* Database port")
            .default(
                std::env::var("POSTGRES_HOST")
                    .unwrap_or("5432".into())
                    .parse::<u32>()
                    .unwrap_or(5432),
            )
            .interact_text()?;
        let postgres_db_name: String = Input::new()
            .with_prompt("* Enter the database name")
            .default(std::env::var("POSTGRES_DB").unwrap_or("chatapp".into()))
            .interact_text()?;
        let postgres_user_name: String = Input::new()
            .with_prompt("* Enter the database user name")
            .default(std::env::var("POSTGRES_USER").unwrap_or("chatter".into()))
            .interact_text()?;

        let (postgres_password, jwt_secret, refresh_secret): (String, String, String) = match (
            std::env::var("POSTGRES_PASSWORD"),
            std::env::var("JWT_SECRET"),
            std::env::var("REFRESH_SECRET"),
        ) {
            (Ok(postgres_password), Ok(jwt_secret), Ok(refresh_secret))
                if Input::new()
                    .with_prompt("Values were found for secrets, do you want to keep them ?")
                    .default(true)
                    .interact()? =>
            {
                (postgres_password, jwt_secret, refresh_secret)
            }
            _ => (
                Password::new()
                    .with_prompt("* Enter the DB password")
                    .interact()?,
                Password::new()
                    .with_prompt("* Enter the JWT password")
                    .interact()?,
                Password::new()
                    .with_prompt("* Enter the refresh token password")
                    .interact()?,
            ),
        };

        let redis_host: String = Input::new()
            .with_prompt("* Enter the redis host")
            .default(std::env::var("REDIS_HOST").unwrap_or("localhost".into()))
            .interact_text()?;

        let redis_port: String = Input::new()
            .with_prompt("* Enter the redis port")
            .default(std::env::var("REDIS_PORT").unwrap_or("6379".into()))
            .interact_text()?;

        let (ssl_certificate_path, ssl_certificate_key, ssl_dhparam_path): (
            Option<String>,
            Option<String>,
            Option<String>,
        ) = match (
            std::env::var("SSL_CERTIFICATE_PATH"),
            std::env::var("SSL_CERTIFICATE_KEY"),
            std::env::var("SSL_DHPARAM_PATH"),
        ) {
            (Ok(ssl_certificate_path), Ok(ssl_certificate_key), Ok(ssl_dhparam_path)) if Input::new().with_prompt(format!("The HTTPs config is the following one so far :\nSSL cert. path : {ssl_certificate_path}\nSSL cert. key : {ssl_certificate_key}\nSSL DH param path : {ssl_dhparam_path}\nDo you want to keep these values ?")).default(true).interact()? =>  (Some(ssl_certificate_path), Some(ssl_certificate_key), Some(ssl_dhparam_path)),
            _ if !Input::new()
            .with_prompt("* Do you want to configure SSL for nginx ?")
                 .default(false)
                 .interact()? => (None, None, None),
            _ =>
            {
                println!("Be careful, the values you will type next have to be either relative or absolute path, otherwise the config won't be understood.\nFor instance, if one of the files is located in this folder, write './myfile' rather than 'myfile'.\n");
                (
                Some(
                    Input::new()
                        .with_prompt("* Indicate the SSL certificate path")
                        .interact()?,
                ),
                Some(
                    Input::new()
                        .with_prompt("* Indicate the SSL key path")
                        .interact()?,
                ),
                Some(
                    Input::new()
                        .with_prompt("* Indicate the SSL dh param file path")
                        .interact()?,
                ),
            )}
        };

        let mut env_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(FILE_NAME)?;

        env_file.write_all(
            EnvTemplate {
                postgres_host,
                postgres_port,
                postgres_db_name,
                postgres_user_name,
                postgres_password,
                jwt_secret,
                refresh_token_secret: refresh_secret,
                ssl_certificate_key,
                ssl_certificate_path,
                ssl_dhparam_path,
                redis_host,
                redis_port,
            }
            .render()?
            .as_bytes(),
        )?;

        Ok(())
    }

    /// Check the current .env file.
    pub async fn check_setup() -> Result<(), CliError> {
        let mut errors: Vec<EnvironmentCheckErrorTypes> = Vec::new();

        println!("\t---\n\tEnvironment variables\n\t---");
        for (env_var, err) in ENV_VARS_TO_CHECK {
            let emoji = match var(env_var).is_ok() {
                true => CHECKMARK_EMOJI,
                false => {
                    let emoji = match err {
                        EnvironmentCheckErrorTypes::Warning => WARNING_EMOJI,
                        EnvironmentCheckErrorTypes::Error => ERROR_EMOJI,
                    };
                    errors.push(err);
                    emoji
                }
            };
            println!("- [{emoji}] {env_var} set in ENV");
        }

        println!("\t---\n\tPrograms in path\n\t---");
        for pgm in PATH_PGM_TO_CHECK {
            let emoji: &str = match which(pgm).is_ok() {
                true => CHECKMARK_EMOJI,
                false => {
                    errors.push(EnvironmentCheckErrorTypes::Warning);
                    WARNING_EMOJI
                }
            };
            println!("- [{emoji}] {pgm}");
        }

        println!("\t---\n\tChecking cargo targets\n\t---");
        let targets: Output = Command::new("rustup")
            .arg("target")
            .arg("list")
            .arg("--installed")
            .output()?;

        match targets.status.success() {
            true => match std::str::from_utf8(&targets.stdout) {
                Ok(v) => {
                    for target in TARGETS_TO_CHECK {
                        let emoji = match v.contains(target) {
                            true => CHECKMARK_EMOJI,
                            false => {
                                errors.push(EnvironmentCheckErrorTypes::Warning);
                                WARNING_EMOJI
                            }
                        };
                        println!("- [{emoji}] {target}");
                    }
                }
                Err(_) => {
                    errors.push(EnvironmentCheckErrorTypes::Warning);
                    println!("- [{WARNING_EMOJI}] Couldn't get the available rustup targets.");
                }
            },
            false => {
                errors.push(EnvironmentCheckErrorTypes::Warning);
                println!("- [{WARNING_EMOJI}] Couldn't get the available rustup targets.");
            }
        }

        println!("\t---\n\tConnection to database\n\t---");
        tchatchers_core::pool::get_pg_pool().await;
        println!("- [{CHECKMARK_EMOJI}] Connection to database");

        println!("\t---\n");

        if errors.is_empty() {
            println!(
                "- [{CHECKMARK_EMOJI}] This set up is ready for either development or production."
            );
        } else {
            let errors_count = errors.len();
            let fatal_errors = errors
                .into_iter()
                .filter(|e| e == &EnvironmentCheckErrorTypes::Error)
                .count();
            if fatal_errors != 0usize {
                println!("- [{ERROR_EMOJI}] Some fatal errors were detected during the setup, please review it. This application will most likely not start.")
            } else {
                println!("- [{WARNING_EMOJI}] Some errors were detected but none of them were fatal, your app might start but you should ensure first that your .env is set up accordingly to your needs.")
            }
            println!("{fatal_errors} Fatal errors detected");
            println!("{} Warnings detected", errors_count - fatal_errors);
        }

        Ok(())
    }

    /// Build a Nginx config file for production usage.
    ///
    /// This can be helpful when you want to for instance change the domain name or disable HTTPS only mode.
    pub(crate) fn build_nginx_conf(output_file: &OsString) -> Result<(), CliError> {
        if fs::read(output_file).is_ok() && !Input::new().with_prompt(format!("Be careful, the output file located at {output_file:?} is non empty, please confirm you want to override the configuration.")).default(false).interact_text()? {
            return Ok(());
        }

        let ssl_certificate = std::env::var("SSL_CERTIFICATE_PATH").ok();
        let ssl_certificate_key: Option<String> = std::env::var("SSL_CERTIFICATE_KEY").ok();
        let ssl_dhparam_path: Option<String> = std::env::var("SSL_DHPARAM_PATH").ok();

        let http_only: bool = if let (
            Some(ssl_certificate),
            Some(ssl_certificate_key),
            Some(ssl_dhparam_path),
        ) = (ssl_certificate, ssl_certificate_key, ssl_dhparam_path)
        {
            Input::new()
                .with_prompt(format!("* Disable https ?\nThis can for instance be helpful if you want to run the project only in DEV mode.\nThe HTTPs config is the following one so far :\nSSL cert. path : {ssl_certificate}\nSSL cert. key : {ssl_certificate_key}\nSSL DH param path : {ssl_dhparam_path}\nDo you want to switch in HTTP only mode ?"))
                .default(false)
                .interact_text()?
        } else {
            println!("* HTTPs mode is not available since .env doesn't contain neither SSL_CERTIFICATE_PATH nor SSL_CERTIFICATE_KEY values.");
            true
        };
        let disable_security: bool = Input::new()
            .with_prompt("* Disable security options ?")
            .default(false)
            .interact_text()?;

        let disable_http2: bool = Input::new()
            .with_prompt("* Disable http2 ?")
            .default(false)
            .interact_text()?;

        let version: String = Input::new()
            .with_prompt("* What is the version of the tool?")
            .interact_text()?;

        let server_name: String = Input::new()
            .with_prompt("* What is the server name ?")
            .default("www.tchatche.rs".into())
            .interact_text()?;

        let mut nginx_config_output = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_file)?;

        let output: String = NginxConfigTemplate {
            http_only,
            disable_security,
            disable_http2,
            version,
            server_name,
        }
        .render()?;
        nginx_config_output.write_all(output.as_bytes())?;
        Ok(())
    }
}
