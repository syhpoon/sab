use orion::aead::SecretKey;

pub fn gen_key() -> String {
    hex::encode(SecretKey::default().unprotected_as_bytes())
}

pub fn cmd_gen_key() {
    println!("{}", gen_key());
}
