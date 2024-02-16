#![warn(clippy::all, clippy::pedantic)]

use std::fmt::Write;
use std::path::PathBuf;

use anyhow::Result;
use indicatif::{ProgressState, ProgressStyle};
use reqwest::Url;
use tokio::sync::mpsc;

mod epg_station_api;
use epg_station_api::api::Client;
use epg_station_api::model::{RecordedQuery, VideoFileProperty, TransferProgress};

mod encoder;
use encoder::model::EncodeProgress;

fn generate_transport_progress_bar() -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(1);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent}%, {bytes_per_sec} (ETA: {eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    pb
}

fn generate_encode_progress_bar() -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(1);
    pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent}%, {per_sec} (ETA: {eta})",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap();
            })
            .progress_chars("#>-"),
        );

    pb
}

#[tokio::main]
async fn main() -> Result<()> {
    let epg_client = Client::new(Url::parse("http://192.168.0.17:8888")?);

    let response = epg_client
        .query_recorded(&RecordedQuery::new(false), 0, 1_000_000)
        .await?;

    let records: Vec<_> = response
        .iter()
        .filter(|record| {
            record.video_files.iter().all(|file| file.type_ == "ts")
                && record.video_files.len() == 1
        })
        .map(|record| {
            (
                record.id,
                record.video_files[0].id,
                &record.video_files[0].filename,
                &record.name,
            )
        })
        .collect();

    // let records: Vec<_> = response
    //     .iter()
    //     .filter(|record| record.id == 164)
    //     .map(|record| {
    //         (
    //             record.id,
    //             record.video_files[0].id,
    //             &record.video_files[0].filename,
    //             &record.name,
    //         )
    //     })
    //     .collect();

    for (record_id, file_id, file_name, name) in records {
        let ts_file_path = PathBuf::from(format!("./{file_name}"));
        let mp4_file_path = PathBuf::from(format!(
            "./{}.mp4",
            ts_file_path
                .with_extension("")
                .file_name()
                .unwrap()
                .to_string_lossy()
        ));

        println!("Downloading {name}...");

        let pb = generate_transport_progress_bar();
        let (tx, mut rx) = mpsc::channel::<TransferProgress>(1);

        tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                pb.set_length(p.total_bytes());
                pb.set_position(p.current_bytes());
            }
            pb.finish_and_clear();
        });

        epg_client
            .download_videofile(file_id, &ts_file_path, tx)
            .await?;

        println!("Encoding {name}...");

        let pb = generate_encode_progress_bar();
        let (tx, mut rx) = mpsc::channel::<EncodeProgress>(1);

        tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                pb.set_length(p.total_secs());
                pb.set_position(p.current_secs());
            }
            pb.finish_and_clear();
        });

        encoder::encode_video_file(&ts_file_path, &mp4_file_path, tx).await?;

        println!("Uploading {name}...");

        let pb = generate_transport_progress_bar();
        let (tx, mut rx) = mpsc::channel::<TransferProgress>(1);

        tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                pb.set_length(p.total_bytes());
                pb.set_position(p.current_bytes());
            }
            pb.finish();
        });

        epg_client
            .upload_videofile(
                &mp4_file_path,
                VideoFileProperty {
                    file_name: mp4_file_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    recorded_id: record_id,
                    parent_directory_name: "recorded".to_string(),
                    sub_directory: None,
                    view_name: "AV1".to_string(),
                    file_type: "encoded".to_string(),
                },
                record_id,
                tx,
            )
            .await?;

        println!("Deleting temp files...");
        tokio::fs::remove_file(&ts_file_path).await?;
        tokio::fs::remove_file(&mp4_file_path).await?;
    }

    Ok(())
}
