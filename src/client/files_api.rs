use std::path::{Component, Path, PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::{Body, Method};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::io::ReaderStream;

use crate::client::FastS3Client;
use crate::encoding::{
    encode_object_key, encode_path_segment, parse_content_disposition_filename,
};
use crate::error::{FastS3Error, Result};
use crate::models::{
    DownloadOptions, DownloadedFile, FileDetail, FileListFilter, JsonUploadRequest,
    StatusMessageResponse, ThumbnailOptions, ThumbnailResponse,
};

impl FastS3Client {
    /// Lists object keys in `bucket` with optional `name` / `subpath` filtering (`GET /api/buckets/{bucket}/files`).
    pub async fn list_files(
        &self,
        bucket: &str,
        filter: Option<&FileListFilter>,
    ) -> Result<Vec<String>> {
        let encoded_bkt = encode_path_segment(bucket);
        let mut req = self.request(Method::GET, &format!("/api/buckets/{encoded_bkt}/files"))?;
        if let Some(f) = filter {
            let pairs = f.to_query_pairs();
            if !pairs.is_empty() {
                req = req.query(&pairs);
            }
        }
        Self::send_json(req).await
    }

    /// Lists detailed file metadata (size, MIME type, extension, `is_image`) in `bucket`
    /// (`GET /api/buckets/{bucket}/files/details`).
    pub async fn list_file_details(
        &self,
        bucket: &str,
        filter: Option<&FileListFilter>,
    ) -> Result<Vec<FileDetail>> {
        let encoded_bkt = encode_path_segment(bucket);
        let mut req = self.request(
            Method::GET,
            &format!("/api/buckets/{encoded_bkt}/files/details"),
        )?;
        if let Some(f) = filter {
            let pairs = f.to_query_pairs();
            if !pairs.is_empty() {
                req = req.query(&pairs);
            }
        }
        Self::send_json(req).await
    }

    /// Uploads in-memory binary data to `PUT /api/buckets/{bucket}/files/{key}`.
    pub async fn upload_bytes(
        &self,
        bucket: &str,
        key: &str,
        data: impl Into<Bytes>,
        content_type: Option<&str>,
    ) -> Result<StatusMessageResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        let bytes: Bytes = data.into();

        let mut req = self
            .request(
                Method::PUT,
                &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}"),
            )?
            .header("Content-Length", bytes.len())
            .body(bytes);

        if let Some(ct) = content_type {
            req = req.header("Content-Type", ct);
        }

        Self::send_json(req).await
    }

    /// Streams an asynchronous byte stream directly to `PUT /api/buckets/{bucket}/files/{key}`
    /// without buffering the entire payload in memory.
    pub async fn upload_stream<S, E>(
        &self,
        bucket: &str,
        key: &str,
        stream: S,
        content_length: Option<u64>,
        content_type: Option<&str>,
    ) -> Result<StatusMessageResponse>
    where
        S: Stream<Item = std::result::Result<Bytes, E>> + Send + Sync + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;

        let mut req = self
            .request(
                Method::PUT,
                &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}"),
            )?
            .body(Body::wrap_stream(stream));

        if let Some(len) = content_length {
            req = req.header("Content-Length", len);
        }
        if let Some(ct) = content_type {
            req = req.header("Content-Type", ct);
        }

        Self::send_json(req).await
    }

    /// Streams a local file from disk to `PUT /api/buckets/{bucket}/files/{key}`.
    pub async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        local_path: impl AsRef<Path>,
    ) -> Result<StatusMessageResponse> {
        let path = local_path.as_ref();
        let file = tokio::fs::File::open(path).await?;
        let meta = file.metadata().await?;
        let len = meta.len();
        let stream = ReaderStream::new(file);
        self.upload_stream(bucket, key, stream, Some(len), None)
            .await
    }

    /// Uploads a file using the JSON + Base64 endpoint (`POST /api/buckets/{bucket}/files/upload`).
    pub async fn upload_json(
        &self,
        bucket: &str,
        payload: &JsonUploadRequest,
    ) -> Result<StatusMessageResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let req = self
            .request(
                Method::POST,
                &format!("/api/buckets/{encoded_bkt}/files/upload"),
            )?
            .json(payload);
        Self::send_json(req).await
    }

    /// Encodes `data` as Base64 and uploads it via `POST /api/buckets/{bucket}/files/upload`.
    pub async fn upload_json_bytes(
        &self,
        bucket: &str,
        key: &str,
        name: Option<&str>,
        data: &[u8],
    ) -> Result<StatusMessageResponse> {
        let payload = JsonUploadRequest {
            name: name.map(ToOwned::to_owned),
            key: key.to_string(),
            data: BASE64_STANDARD.encode(data),
        };
        self.upload_json(bucket, &payload).await
    }

    /// Downloads a file along with its HTTP headers (`GET /api/buckets/{bucket}/files/{key}`).
    pub async fn download_file(&self, bucket: &str, key: &str) -> Result<DownloadedFile> {
        self.download_with_options(bucket, key, &DownloadOptions::default())
            .await
    }

    /// Downloads raw file bytes (`GET /api/buckets/{bucket}/files/{key}`).
    pub async fn download_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let downloaded = self.download_file(bucket, key).await?;
        Ok(downloaded.data.to_vec())
    }

    /// Downloads a file with query options (e.g. `?thumbnail=true&width=...&height=...`).
    pub async fn download_with_options(
        &self,
        bucket: &str,
        key: &str,
        options: &DownloadOptions,
    ) -> Result<DownloadedFile> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;

        let mut req = self.request(
            Method::GET,
            &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}"),
        )?;

        let pairs = options.to_query_pairs();
        if !pairs.is_empty() {
            req = req.query(&pairs);
        }

        let resp = Self::ensure_success(req.send().await?).await?;

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let content_length = resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let filename = resp
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_content_disposition_filename);

        let data = resp.bytes().await?;

        Ok(DownloadedFile {
            data,
            content_type,
            content_length,
            filename,
        })
    }

    /// Streams a file download directly into an [`AsyncWrite`] sink without buffering the whole object in RAM.
    /// Returns the total number of bytes written.
    pub async fn download_to_writer<W>(&self, bucket: &str, key: &str, writer: &mut W) -> Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;

        let req = self.request(
            Method::GET,
            &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}"),
        )?;

        let resp = Self::ensure_success(req.send().await?).await?;
        let mut stream = resp.bytes_stream();
        let mut written: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            writer.write_all(&bytes).await?;
            written += bytes.len() as u64;
        }
        writer.flush().await?;

        Ok(written)
    }

    /// Streams a file download directly to `dest_path` on disk, creating parent directories if needed.
    /// Returns the total number of bytes written.
    pub async fn download_to_path(
        &self,
        bucket: &str,
        key: &str,
        dest_path: impl AsRef<Path>,
    ) -> Result<u64> {
        let path = dest_path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::File::create(path).await?;
        self.download_to_writer(bucket, key, &mut file).await
    }

    /// Retrieves a resized image thumbnail via `GET /api/buckets/{bucket}/files/{key}/thumbnail`.
    pub async fn get_thumbnail(
        &self,
        bucket: &str,
        key: &str,
        options: Option<&ThumbnailOptions>,
    ) -> Result<ThumbnailResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        self.fetch_thumbnail_route(
            &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}/thumbnail"),
            options,
        )
        .await
    }

    /// Retrieves a resized image thumbnail via the alternate route `GET /api/buckets/{bucket}/thumbnail/{key}`.
    pub async fn get_thumbnail_alt(
        &self,
        bucket: &str,
        key: &str,
        options: Option<&ThumbnailOptions>,
    ) -> Result<ThumbnailResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        self.fetch_thumbnail_route(
            &format!("/api/buckets/{encoded_bkt}/thumbnail/{encoded_key}"),
            options,
        )
        .await
    }

    async fn fetch_thumbnail_route(
        &self,
        route: &str,
        options: Option<&ThumbnailOptions>,
    ) -> Result<ThumbnailResponse> {
        let mut req = self.request(Method::GET, route)?;
        if let Some(opts) = options {
            let pairs = opts.to_query_pairs();
            if !pairs.is_empty() {
                req = req.query(&pairs);
            }
        }

        let resp = Self::ensure_success(req.send().await?).await?;
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let data = resp.bytes().await?;

        Ok(ThumbnailResponse { data, content_type })
    }

    /// Downloads a thumbnail and writes it to `dest_path`.
    pub async fn save_thumbnail_to_path(
        &self,
        bucket: &str,
        key: &str,
        options: Option<&ThumbnailOptions>,
        dest_path: impl AsRef<Path>,
    ) -> Result<()> {
        let thumb = self.get_thumbnail(bucket, key, options).await?;
        let path = dest_path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, &thumb.data).await?;
        Ok(())
    }

    /// Deletes an object from a bucket (`DELETE /api/buckets/{bucket}/files/{key}`).
    pub async fn delete_file(&self, bucket: &str, key: &str) -> Result<StatusMessageResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        let req = self.request(
            Method::DELETE,
            &format!("/api/buckets/{encoded_bkt}/files/{encoded_key}"),
        )?;
        Self::send_json(req).await
    }

    /// Recursively uploads all files in `local_dir` into `bucket` under optional `key_prefix`.
    /// Returns the list of uploaded S3 object keys.
    pub async fn upload_directory(
        &self,
        bucket: &str,
        key_prefix: Option<&str>,
        local_dir: impl AsRef<Path>,
    ) -> Result<Vec<String>> {
        let root = local_dir.as_ref();
        if !root.is_dir() {
            return Err(FastS3Error::InvalidArgument(format!(
                "Path '{}' is not a directory",
                root.display()
            )));
        }

        let clean_prefix = key_prefix
            .unwrap_or("")
            .replace('\\', "/")
            .trim_matches('/')
            .to_string();

        let mut stack = vec![root.to_path_buf()];
        let mut uploaded_keys = Vec::new();

        while let Some(dir) = stack.pop() {
            let mut entries = tokio::fs::read_dir(&dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    stack.push(path);
                } else if file_type.is_file() {
                    let rel = path
                        .strip_prefix(root)
                        .map_err(|e| FastS3Error::InvalidArgument(e.to_string()))?;
                    let rel_key = rel.to_string_lossy().replace('\\', "/");
                    let full_key = if clean_prefix.is_empty() {
                        rel_key
                    } else {
                        format!("{clean_prefix}/{rel_key}")
                    };
                    self.upload_file(bucket, &full_key, &path).await?;
                    uploaded_keys.push(full_key);
                }
            }
        }

        uploaded_keys.sort();
        Ok(uploaded_keys)
    }

    /// Downloads all objects matching `subpath` (or the entire bucket when `None`) into `dest_dir`,
    /// preserving their relative folder hierarchy safely. Returns the written local file paths.
    pub async fn download_prefix_to_dir(
        &self,
        bucket: &str,
        subpath: Option<&str>,
        dest_dir: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>> {
        let base_dest = dest_dir.as_ref();
        tokio::fs::create_dir_all(base_dest).await?;

        let filter = subpath.map(|s| FileListFilter::new().with_subpath(s));
        let keys = self.list_files(bucket, filter.as_ref()).await?;
        let mut written_paths = Vec::with_capacity(keys.len());

        for key in keys {
            let mut safe_rel = PathBuf::new();
            for comp in Path::new(&key).components() {
                if let Component::Normal(part) = comp {
                    safe_rel.push(part);
                }
            }
            if safe_rel.as_os_str().is_empty() {
                continue;
            }
            let target = base_dest.join(safe_rel);
            self.download_to_path(bucket, &key, &target).await?;
            written_paths.push(target);
        }

        Ok(written_paths)
    }

    /// Deletes all objects under `subpath` in `bucket`. Returns the number of deleted objects.
    pub async fn delete_prefix(&self, bucket: &str, subpath: &str) -> Result<usize> {
        let filter = FileListFilter::new().with_subpath(subpath);
        let keys = self.list_files(bucket, Some(&filter)).await?;
        let count = keys.len();
        for key in keys {
            self.delete_file(bucket, &key).await?;
        }
        Ok(count)
    }

    /// Builds a direct HTTP URL for `GET /api/buckets/{bucket}/files/{key}`.
    ///
    /// When `include_auth_query` is `true`, appends FastS3's query credential parameter
    /// (`?token=...`, `?api_key=...`, or `?auth=...`) for direct browser/embed links on private buckets.
    pub fn file_url(&self, bucket: &str, key: &str, include_auth_query: bool) -> Result<String> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        let base = format!("{}/api/buckets/{encoded_bkt}/files/{encoded_key}", self.base_url);

        if include_auth_query && let Some((k, v)) = self.credentials().to_query_param() {
            return Ok(format!("{base}?{k}={v}"));
        }
        Ok(base)
    }

    /// Builds a direct HTTP URL for `GET /api/buckets/{bucket}/files/{key}/thumbnail`.
    pub fn thumbnail_url(
        &self,
        bucket: &str,
        key: &str,
        options: Option<&ThumbnailOptions>,
        include_auth_query: bool,
    ) -> Result<String> {
        let encoded_bkt = encode_path_segment(bucket);
        let encoded_key = encode_object_key(key)?;
        let base = format!(
            "{}/api/buckets/{encoded_bkt}/files/{encoded_key}/thumbnail",
            self.base_url
        );

        let mut params: Vec<String> = Vec::new();
        if let Some(opts) = options {
            for (k, v) in opts.to_query_pairs() {
                params.push(format!("{k}={v}"));
            }
        }
        if include_auth_query && let Some((k, v)) = self.credentials().to_query_param() {
            params.push(format!("{k}={v}"));
        }

        if params.is_empty() {
            Ok(base)
        } else {
            Ok(format!("{base}?{}", params.join("&")))
        }
    }
}
