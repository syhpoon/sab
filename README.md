# sab

`sab` is a simple backup tool that uses AWS S3:

* simple interface to upload/download backup archives
* multiple profiles
* encryption and compression
* `STANDARD` and `DEEP_ARCHIVE` (Glacier) storage classes

# Installation

`cargo install sab`

# Usage

## Create a profile

```shell
$ sab init
S3 Access Key: MY-ACCESS-KEY
S3 Secret Key: MY-SECRET-KEY
S3 Region [us-east-1]:
Bucket Name: my-backups
Bucket Prefix for Backups []: laptop/
Enable Encryption? [true]:
```

## Upload a file

```shell
$ sab upload backup.tar.bz2 -s 50MB
[2023-01-22T05:09:24Z INFO  sab::cli::cmd_upload] starting upload laptop/backup.tar.bz2
[2023-01-22T05:09:24Z INFO  sab::cli::cmd_upload] creating new configuration
[2023-01-22T05:09:51Z INFO  sab::cli::cmd_upload] uploaded chunk=1	orig-size=50000000	processed-size=50000040	progress=30.16%
[2023-01-22T05:10:17Z INFO  sab::cli::cmd_upload] uploaded chunk=2	orig-size=50000000	processed-size=50000040	progress=60.31%
[2023-01-22T05:10:45Z INFO  sab::cli::cmd_upload] uploaded chunk=3	orig-size=50000000	processed-size=50000040	progress=90.47%
[2023-01-22T05:10:53Z INFO  sab::cli::cmd_upload] uploaded chunk=4	orig-size=15805568	processed-size=15805608	progress=100.00%
[2023-01-22T05:10:53Z INFO  sab::cli::cmd_upload] upload completed
```

## List backups

```shell
$  sab list
* laptop/backup.tar.bz2
```

## Download backup

```shell
$ sab download backup.tar.bz2
[2023-01-22T05:13:20Z INFO  sab::cli::cmd_download] starting download
[2023-01-22T05:13:32Z INFO  sab::cli::cmd_download] downloaded chunk=1	size=50000000	progress=30.16%
[2023-01-22T05:13:45Z INFO  sab::cli::cmd_download] downloaded chunk=2	size=50000000	progress=60.31%
[2023-01-22T05:13:58Z INFO  sab::cli::cmd_download] downloaded chunk=3	size=50000000	progress=90.47%
[2023-01-22T05:14:02Z INFO  sab::cli::cmd_download] downloaded chunk=4	size=15805568	progress=100.00%
[2023-01-22T05:14:02Z INFO  sab::cli::cmd_download] backup successfully downloaded
```

Note, that if `DEEP_ARCHIVE` storage class was used when uploading a backup,
the file needs to be [restored](https://docs.aws.amazon.com/AmazonS3/latest/userguide/restoring-objects.html) in AWS before it can be downloaded.