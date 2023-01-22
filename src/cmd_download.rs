use std::fs::File;
use std::io::{Read, Write};

use crate::config::{Backup, Config};
use crate::s3::S3Client;

use flate2::read::GzDecoder;
use orion::aead;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

pub async fn cmd_download(
    cl: S3Client<'_>,
    name: &str,
    out_file: &str,
    encryption_key: Vec<u8>,
    cfg: &Config,
) {
    let backup_file = cfg.backup(name);
    if !backup_file.exists() {
        panic!("no backup named {}", name);
    }

    let backup = Backup::load(backup_file.as_path()).expect("failed to load backup");

    if !backup.done {
        panic!("backup is not completed!");
    }

    let mut reader = cl
        .download(&backup)
        .await
        .expect("failed to obtain download reader");

    let mut f = File::create(out_file).expect("failed to create the output file");

    log::info!("starting download");

    let total_size: f64 = backup
        .parts
        .iter()
        .map(|part| part.original_size as f64)
        .sum();

    let mut downloaded_size: f64 = 0.;
    let mut hasher = Sha256::new();

    for part in backup.parts.iter() {
        let mut chunk_processed_hasher = Sha256::new();
        let mut chunk_orig_hasher = Sha256::new();
        let mut buf = vec![0u8; part.processed_size as usize];

        let _ = reader
            .read_exact(buf.as_mut_slice())
            .await
            .expect("failed to read chunk from the stream");

        chunk_processed_hasher.update(buf.as_slice());
        let processed_hash = hex::encode(chunk_processed_hasher.finalize());

        if processed_hash != part.processed_sha256 {
            panic!(
                "chunk {} processed checksum mismatch, expected={}, got={}",
                part.idx, &part.processed_sha256, &processed_hash
            );
        }

        if backup.encryption_enabled {
            let enc_key = aead::SecretKey::from_slice(encryption_key.as_slice())
                .expect("failed to load encryption key");

            buf = aead::open(&enc_key, buf.as_slice()).expect("failed to decrypt chunk");
        }

        if backup.compression_enabled {
            let mut dec = GzDecoder::new(buf.as_slice());
            let mut dst: Vec<u8> = Vec::new();

            dec.read_to_end(&mut dst).unwrap();
            buf = dst;
        }

        if buf.len() != part.original_size as usize {
            panic!(
                "chunk {} size mismatch, expected={}, got={}",
                part.idx,
                part.original_size,
                buf.len()
            );
        }

        hasher.update(buf.as_slice());
        chunk_orig_hasher.update(buf.as_slice());

        let orig_hash = hex::encode(chunk_orig_hasher.finalize());
        if orig_hash != part.original_sha256 {
            panic!(
                "chunk {} checksum mismatch, expected={}, got={}",
                part.idx, &part.original_sha256, &orig_hash
            );
        }

        f.write_all(buf.as_slice())
            .expect("failed to write part to file");

        downloaded_size += buf.len() as f64;
        let progress = (downloaded_size / total_size) * 100.;

        log::info!(
            "downloaded chunk={}\tsize={}\tprogress={:.2}%",
            part.idx,
            part.original_size,
            progress
        );
    }

    let hash = hex::encode(hasher.finalize());
    if hash != backup.sha256 {
        panic!(
            "backup checksum mismatch, expected={}, got={}",
            &backup.sha256, &hash
        );
    }

    log::info!("backup successfully downloaded");
}
