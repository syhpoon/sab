use crate::config::{Backup, Profile};

use anyhow::Result;
use aws_sdk_s3::model::{CompletedMultipartUpload, CompletedPart, StorageClass};
use aws_sdk_s3::output::CreateMultipartUploadOutput;
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::{Client, Credentials, Region};
use tokio::io::AsyncRead;

pub struct S3Client<'a> {
    cl: Client,
    profile: &'a Profile,
}

impl<'a> S3Client<'a> {
    pub async fn new(profile: &'a Profile) -> S3Client<'a> {
        let creds = Credentials::new(&profile.access_key, &profile.secret_key, None, None, "sab");

        let cfg = aws_config::from_env()
            .region(Region::new(profile.region.to_string()))
            .credentials_provider(creds)
            .load()
            .await;

        let cl = Client::new(&cfg);

        S3Client { cl, profile }
    }

    pub async fn list_uploads(&self) -> Result<Vec<String>> {
        let resp = self
            .cl
            .list_objects_v2()
            .bucket(&self.profile.bucket)
            .prefix(&self.profile.prefix)
            .send()
            .await?;

        let keys = resp
            .contents()
            .unwrap_or_default()
            .iter()
            .map(|item| item.key().unwrap().to_string())
            .collect();

        Ok(keys)
    }

    pub async fn create_upload(&self, name: &str, class: StorageClass) -> Result<String> {
        let res: CreateMultipartUploadOutput = self
            .cl
            .create_multipart_upload()
            .bucket(&self.profile.bucket)
            .key(name)
            .storage_class(class)
            .send()
            .await?;

        Ok(res.upload_id().unwrap().to_string())
    }

    pub async fn upload_chunk(
        &self,
        backup: &Backup,
        part: i32,
        body: ByteStream,
    ) -> Result<String> {
        let res = self
            .cl
            .upload_part()
            .key(&backup.name)
            .bucket(&self.profile.bucket)
            .upload_id(&backup.upload_id)
            .part_number(part)
            .body(body)
            .send()
            .await?;

        Ok(res.e_tag().unwrap().to_string())
    }

    pub async fn finish_upload(&self, backup: &Backup) -> Result<String> {
        let parts: Vec<CompletedPart> = backup
            .parts
            .iter()
            .map(|part| {
                CompletedPart::builder()
                    .e_tag(&part.etag)
                    .part_number(part.idx as i32)
                    .build()
            })
            .collect();

        let upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        let res = self
            .cl
            .complete_multipart_upload()
            .bucket(&self.profile.bucket)
            .key(&backup.name)
            .multipart_upload(upload)
            .upload_id(&backup.upload_id)
            .send()
            .await?;

        Ok(res.e_tag().unwrap().to_string())
    }

    pub async fn download(&self, backup: &Backup) -> Result<impl AsyncRead> {
        let out = self
            .cl
            .get_object()
            .bucket(&self.profile.bucket)
            .key(&backup.name)
            .send()
            .await?;

        Ok(out.body.into_async_read())
    }
}
