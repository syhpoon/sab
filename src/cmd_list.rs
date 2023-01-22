use crate::s3::S3Client;

pub async fn cmd_list(cl: S3Client<'_>) {
    let uploads = cl.list_uploads().await.expect("failed to list uploads");

    uploads.iter().for_each(|upload| println!("* {}", upload));
}
