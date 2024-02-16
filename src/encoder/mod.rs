use std::path::Path;

use anyhow::Result;
use ffmpeg_cli::{FfmpegBuilder, File, Parameter};
use ffprobe::ffprobe;
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub mod model;
use model::EncodeProgress;

pub async fn encode_video_file(
    source: &Path,
    target: &Path,
    progress: mpsc::Sender<EncodeProgress>,
) -> Result<()> {
    let source_str = source.to_string_lossy();
    let target_str = target.to_string_lossy();

    let output_options = File::new(&target_str)
        .option(Parameter::KeyValue("map", "0:v"))
        .option(Parameter::KeyValue("map", "0:s?"))
        .option(Parameter::KeyValue("vcodec", "libsvtav1"))
        .option(Parameter::KeyValue("crf", "38"))
        .option(Parameter::KeyValue("vf", "yadif=1"))
        .option(Parameter::KeyValue("absf", "aac_adtstoasc"))
        .option(Parameter::KeyValue("fflags", "+discardcorrupt"))
        .option(Parameter::KeyValue("acodec", "copy"))
        .option(Parameter::KeyValue("scodec", "mov_text"));

    let source_props = ffprobe(source)?;

    let map_options: Vec<_> = source_props
        .streams
        .into_iter()
        .enumerate()
        .filter_map(|(n, s)| {
            let Some(codec_type) = &s.codec_type else {
                return None;
            };

            if codec_type != "audio" {
                return None;
            }

            match s.channels {
                Some(0) => None,
                Some(_) => Some(format!("0:{n}")),
                None => unreachable!("Audio streams must have a channels property"),
            }
        })
        .collect();

    let output_options = map_options.iter().fold(output_options, |opts, map_s| {
        opts.option(Parameter::KeyValue("map", map_s))
    });

    let builder = FfmpegBuilder::new()
        .option(Parameter::Single("y"))
        .option(Parameter::KeyValue("analyzeduration", "100M"))
        .option(Parameter::KeyValue("probesize", "100M"))
        .option(Parameter::Single("ignore_unknown"))
        .option(Parameter::Single("fix_sub_duration"))
        .input(File::new(&source_str))
        .output(output_options);

    let mut ffmpeg = builder.run().await.unwrap();

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    let total_secs = source_props.format.duration.unwrap().parse::<f64>()? as u64;
    let _ = progress.try_send(EncodeProgress {
        current_secs: 0,
        total_secs,
    });

    while let Some(v) = ffmpeg.progress.next().await {
        let _ = progress.try_send(EncodeProgress {
            current_secs: v.unwrap().out_time.unwrap().as_secs(),
            total_secs,
        });
    }

    ffmpeg.process.wait_with_output()?;

    Ok(())
}
