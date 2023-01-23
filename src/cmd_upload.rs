use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::exit;

use crate::config::{Backup, Config, UploadPart};
use crate::s3::S3Client;

use aws_sdk_s3::model::StorageClass;
use aws_sdk_s3::types::ByteStream;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use orion::aead;
use sha2::{Digest, Sha256};

const MAX_CHUNKS: u64 = 10_000;

pub async fn cmd_upload(
    cl: S3Client<'_>,
    file: &str,
    chunk_size: usize,
    compression_enabled: bool,
    encryption_enabled: bool,
    encryption_key: Vec<u8>,
    prefix: String,
    class: StorageClass,
    cfg: &Config,
) {
    // Check if there's a pending upload already
    let backup_file = cfg.backup(file);
    let mut backup: Backup;

    let input_file = PathBuf::from(file);
    let md = input_file
        .metadata()
        .expect("failed to get input file metadata");

    let num_chunks = md.size() / chunk_size as u64;
    if num_chunks > MAX_CHUNKS {
        log::error!(
            "the total number of chunks {} exceeds the maximum amount of {}, consider increasing the chunk size", num_chunks, MAX_CHUNKS);
        exit(1);
    }

    let name = input_file
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let key = prefix.to_string() + &name;

    log::info!("starting upload {}", &key);

    if backup_file.exists() {
        log::info!("loading existing configuration");

        backup = Backup::load(backup_file.as_path()).expect("failed to load backup config");
    } else {
        log::info!("creating new configuration");

        let upload_id = cl
            .create_upload(key.as_str(), class)
            .await
            .expect("failed to create upload");

        backup = Backup {
            name: key.clone(),
            prefix: prefix.clone(),
            chunk_size,
            upload_id,
            parts: vec![],
            done: false,
            started: Utc::now().to_string(),
            completed: "".to_string(),
            sha256: "".to_string(),
            compression_enabled,
            encryption_enabled,
        };

        backup
            .save(backup_file.as_path())
            .expect("failed to save backup config");
    }

    let mut f = File::open(file).expect("failed to open upload file");
    let mut idx: usize = 1;
    let existing_parts = backup.parts.len();

    let mut hasher = Sha256::new();
    let total_size = f.metadata().unwrap().len() as f64;
    let mut uploaded_size: f64 = 0.;

    loop {
        let mut buf: Vec<u8> = vec![0u8; chunk_size];
        let size = f.read(&mut buf).expect("failed to read from upload file");
        if size == 0 {
            break;
        }

        let mut chunk_orig_hasher = Sha256::new();
        let mut chunk_processed_hasher = Sha256::new();

        buf.truncate(size);

        // Update hashes
        hasher.update(buf.as_slice());

        if idx <= existing_parts {
            log::info!("chunk {} already uploaded, skipping", idx);
            idx += 1;
            continue;
        }

        chunk_orig_hasher.update(buf.as_slice());

        if compression_enabled {
            let mut enc = GzEncoder::new(Vec::new(), Compression::default());
            enc.write_all(buf.as_slice())
                .expect("failed to compress chunk");
            buf = enc.finish().expect("failed to complete chunk compression");
        }

        if encryption_enabled {
            let enc_key = aead::SecretKey::from_slice(encryption_key.as_slice())
                .expect("failed to load encryption key");

            buf = aead::seal(&enc_key, buf.as_slice()).expect("failed to encrypt chunk");
        }

        chunk_processed_hasher.update(buf.as_slice());

        let processed_size = buf.len() as u64;
        let body = ByteStream::from(buf);
        let etag = cl
            .upload_chunk(&backup, idx as i32, body)
            .await
            .expect("failed to upload chunk");

        uploaded_size += size as f64;
        let progress = (uploaded_size / total_size) * 100.;

        log::info!(
            "uploaded chunk={}\torig-size={}\tprocessed-size={}\tprogress={:.2}%",
            idx,
            size,
            processed_size,
            progress
        );

        backup.parts.push(UploadPart {
            idx,
            etag,
            original_size: size as u64,
            processed_size,
            original_sha256: hex::encode(chunk_orig_hasher.finalize()),
            processed_sha256: hex::encode(chunk_processed_hasher.finalize()),
        });
        backup
            .save(backup_file.as_path())
            .expect("failed to save backup config");

        if size != chunk_size {
            break;
        }

        idx += 1;
    }

    cl.finish_upload(&backup)
        .await
        .expect("failed to finish upload");

    backup.done = true;
    backup.completed = Utc::now().to_string();
    backup.sha256 = hex::encode(hasher.finalize());
    backup
        .save(backup_file.as_path())
        .expect("failed to save backup config");

    log::info!("upload completed");
}
