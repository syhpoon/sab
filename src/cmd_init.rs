use std::fmt::{Debug, Display};
use std::fs;
use std::io::{stdin, stdout, Write};
use std::str::FromStr;

use crate::cmd_gen_key::gen_key;

use crate::config::{Config, Profile};

pub fn cmd_init(profile_name: &str) {
    let sab_dir = Config::sab_dir();
    if !sab_dir.exists() {
        fs::create_dir_all(sab_dir.as_path()).expect("failed to create sab directory");
    }

    let profiles_file = Config::profiles_file();
    let mut cfg = if !profiles_file.exists() {
        Config::default()
    } else {
        Config::load().expect("failed to load profiles config")
    };

    if let Some(_) = cfg.profile(profile_name) {
        panic!("profile {} already exists", profile_name);
    }

    let mut profile = Profile::default();
    populate_profile(&mut profile);

    cfg.set_profile(profile_name, profile);
    cfg.save().expect("failed to save profiles config");
}

fn populate_profile(profile: &mut Profile) {
    profile.access_key = input("S3 Access Key");
    profile.secret_key = input("S3 Secret Key");
    profile.region = input_default("S3 Region", "us-east-1".to_string());
    profile.bucket = input("Bucket Name");
    profile.prefix = input_default("Bucket Prefix for Backups", "".to_string());

    let enc_enabled = input_default("Enable Encryption?", true);
    if enc_enabled {
        profile.encryption_key = gen_key();
    }
}

fn input(prompt: &str) -> String {
    loop {
        print!("{}: ", prompt);
        let _ = stdout().flush();
        let mut val = String::new();
        stdin()
            .read_line(&mut val)
            .expect("failed to read input line");

        val = val.trim_end().to_string();
        if val.is_empty() {
            continue;
        }

        return val;
    }
}

fn input_default<T>(prompt: &str, def: T) -> T
where
    T: Display + FromStr,
    <T as FromStr>::Err: Debug,
{
    print!("{} [{}]:", prompt, def);
    let _ = stdout().flush();
    let mut val = String::new();
    stdin()
        .read_line(&mut val)
        .expect("failed to read input line");
    val = val.trim_end().to_string();

    if val.is_empty() {
        def
    } else {
        val.parse().expect("failed to parse value")
    }
}
