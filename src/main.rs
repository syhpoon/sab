extern crate core;
extern crate log;

mod config;
mod s3;
mod cmd_download;
mod cmd_init;
mod cmd_list;
mod cmd_upload;
mod cmd_gen_key;

use config::Config;
use s3::S3Client;

use cmd_download::cmd_download;
use cmd_init::cmd_init;
use cmd_list::cmd_list;
use cmd_upload::cmd_upload;
use cmd_gen_key::cmd_gen_key;

use aws_sdk_s3::model::StorageClass;
use clap::{ArgAction, Parser, Subcommand};
use humanize_rs::bytes::Bytes;

#[derive(Parser)]
#[command(author, about, version, long_about=None)]
pub struct Cli {
    #[arg(
        short = 'p',
        long = "profile",
        global = true,
        default_value = "default"
    )]
    profile: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Init a new profile")]
    Init { name: Option<String> },
    #[command(about = "List uploads")]
    List {},
    #[command(about = "Generate an encryption key")]
    GenKey {},
    #[command(about = "Create new or resume existing upload")]
    Upload {
        file: String,

        #[arg(short = 's', long = "chunk-size", default_value = "100MB")]
        chunk_size: String,

        #[arg(short='c', long="with-compression", action=ArgAction::Set, default_value_t=false)]
        compression_enabled: bool,

        #[arg(short='e', long="with-encryption", action=ArgAction::Set, default_value_t=true)]
        encryption_enabled: bool,

        #[arg(short='l', long="storage-class", default_value="STANDARD",
              value_parser=["STANDARD", "DEEP_ARCHIVE"])]
        storage_class: String,
    },
    #[command(about = "Download a file from the archive")]
    Download {
        name: String,
        output_file: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let cli = Cli::parse();

    match cli.command {
        Commands::GenKey {} => {
            cmd_gen_key();
        }
        Commands::Init { name } => {
            let name = name.unwrap_or("default".to_string());
            cmd_init(&name);
        }
        Commands::List {} => {
            let cfg = load_config();
            let profile = cfg.profile(&cli.profile).expect("unknown profile");
            let cl = S3Client::new(profile).await;

            cmd_list(cl).await;
        }
        Commands::Upload {
            file,
            chunk_size,
            compression_enabled,
            encryption_enabled,
            storage_class,
        } => {
            let cfg = load_config();
            let profile = cfg.profile(&cli.profile).expect("unknown profile");
            let cl = S3Client::new(profile).await;
            let size = chunk_size
                .parse::<Bytes>()
                .expect("failed to parse chunk size");

            let enc_key =
                hex::decode(&profile.encryption_key).expect("failed to hex decode encryption key");

            let class = StorageClass::from(storage_class.as_str());
            cmd_upload(
                cl,
                &file,
                size.size(),
                compression_enabled,
                encryption_enabled,
                enc_key,
                profile.prefix.to_string(),
                class,
                &cfg,
            )
            .await;
        }
        Commands::Download { name, output_file } => {
            let cfg = load_config();
            let profile = cfg.profile(&cli.profile).expect("unknown profile");
            let cl = S3Client::new(profile).await;

            let enc_key =
                hex::decode(&profile.encryption_key).expect("failed to hex decode encryption key");

            let out = output_file.unwrap_or(name.to_string());
            cmd_download(cl, &name, &out, enc_key, &cfg).await;
        }
    }
}

fn load_config() -> Config {
    Config::load().expect("failed to load config")
}
