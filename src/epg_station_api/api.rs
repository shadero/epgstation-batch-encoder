use std::path::Path;

use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Url;
use tokio::{io::AsyncWriteExt, sync::mpsc};

use super::model::{
    Record, RecordId, RecordedEndpointResponse, RecordedQuery, VideoFileId, VideoFileProperty,
};

pub struct TransferProgress {
    pub(in crate::epg_station_api) total_bytes: u64,
    pub(in crate::epg_station_api) current_bytes: u64,
}

impl TransferProgress {
    fn new(total: u64, current: u64) -> Self {
        Self {
            total_bytes: total,
            current_bytes: current,
        }
    }

    pub fn current_bytes(&self) -> u64 {
        self.current_bytes
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

pub struct Client {
    host: Url,
}

impl Client {
    pub fn new(base_uri: Url) -> Self {
        Self { host: base_uri }
    }

    pub async fn query_recorded(
        &self,
        query: &RecordedQuery,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Record>> {
        let mut url = self.host.clone();
        url.set_path("/api/recorded");

        url.query_pairs_mut()
            .clear()
            .extend_pairs(&query.to_parameters())
            .append_pair("offset", &offset.to_string())
            .append_pair("limit", &limit.to_string());

        let response: RecordedEndpointResponse =
            reqwest::get(url).await?.error_for_status()?.json().await?;

        Ok(response.records)
    }

    pub async fn download_videofile(
        &self,
        videofile_id: VideoFileId,
        target: &Path,
        progress: mpsc::Sender<TransferProgress>,
    ) -> Result<()> {
        let mut url = self.host.clone();
        url.set_path(&format!("/api/videos/{}", videofile_id));

        let response = reqwest::get(url).await?;
        let byte_size: u64 = response
            .headers()
            .get("content-length")
            .unwrap()
            .to_str()?
            .parse()?;
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(target).await?;

        let mut current_byte_size = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = &chunk?;
            file.write_all(&chunk).await?;
            current_byte_size += chunk.len() as u64;
            let _ = progress.try_send(TransferProgress::new(byte_size, current_byte_size));
        }

        Ok(())
    }

    pub async fn upload_videofile(
        &self,
        upload_video_file_path: &Path,
        property: VideoFileProperty,
        record_id: RecordId,
        progress: mpsc::Sender<TransferProgress>,
    ) -> Result<()> {
        let mut url = self.host.clone();
        url.set_path("/api/videos/upload");

        let file = tokio::fs::File::open(upload_video_file_path).await?;
        let file_size = file.metadata().await?.len();
        let mut sent_bytes = 0;

        let file = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file).inspect(
            move |buf| match buf {
                Ok(buf) => {
                    sent_bytes += buf.len() as u64;
                    let _ = progress.try_send(TransferProgress::new(file_size, sent_bytes));
                }
                Err(_) => {}
            },
        ));

        let file = reqwest::multipart::Part::stream(file)
            .file_name(property.file_name)
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = reqwest::multipart::Form::new()
            .text("recordedId", record_id.to_string())
            .text("parentDirectoryName", property.parent_directory_name)
            .text("viewName", property.view_name)
            .text("fileType", property.file_type)
            .part("file", file);

        if let Some(sub_directory) = property.sub_directory {
            form = form.text("subDirectory", sub_directory);
        }

        reqwest::Client::new()
            .post(url)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
