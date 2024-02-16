use serde::Deserialize;

pub type RecordId = u64;
pub type VideoFileId = u64;
pub type ChannelId = u64;
pub type RuleId = u64;
pub type ProgramId = u64;
pub type ThumbnailId = u64;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub id: VideoFileId,
    pub name: String,
    pub filename: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: RecordId,
    pub channel_id: ChannelId,
    pub start_at: u64,
    pub end_at: u64,
    pub name: String,
    pub is_recording: bool,
    pub is_encoding: bool,
    pub is_protected: bool,
    pub rule_id: Option<RuleId>,
    pub program_id: ProgramId,
    pub description: String,
    pub extended: Option<String>,
    pub genre1: u16,
    pub sub_genre1: u16,
    pub video_type: String,
    pub video_resolution: String,
    pub video_stream_content: i32,
    pub video_component_type: i32,
    pub audio_sampling_rate: u32,
    pub audio_component_type: u8,
    pub thumbnails: Vec<ThumbnailId>,
    pub video_files: Vec<VideoFile>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(in crate::epg_station_api) struct RecordedEndpointResponse {
    pub(in crate::epg_station_api) records: Vec<Record>,

    #[allow(dead_code)]
    pub(in crate::epg_station_api) total: usize,
}

#[derive(Clone, Debug)]
pub struct VideoFileProperty {
    pub file_name: String,
    pub recorded_id: RecordId,
    pub parent_directory_name: String,
    pub sub_directory: Option<String>,
    pub view_name: String,
    pub file_type: String,
}

#[derive(Debug)]
pub struct RecordedQuery {
    is_half_width: bool,
    is_reverse: Option<bool>,
    rule_id: Option<RuleId>,
    channel_id: Option<ChannelId>,
}

impl RecordedQuery {
    pub fn new(is_half_width: bool) -> Self {
        Self {
            is_half_width,
            is_reverse: None,
            rule_id: None,
            channel_id: None,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    #[allow(dead_code)]
    pub fn is_reverse(mut self, is_reverse: bool) -> Self {
        self.is_reverse = Some(is_reverse);
        self
    }

    #[allow(dead_code)]
    pub fn rule_id(mut self, rule_id: RuleId) -> Self {
        self.rule_id = Some(rule_id);
        self
    }

    #[allow(dead_code)]
    pub fn channel_id(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    pub(in crate::epg_station_api) fn to_parameters(&self) -> Vec<(String, String)> {
        let mut parameters = vec![];

        parameters.push(("isHalfWidth".to_string(), self.is_half_width.to_string()));

        if let Some(rule_id) = self.rule_id {
            parameters.push(("ruleId".to_string(), rule_id.to_string()));
        }

        if let Some(channel_id) = self.channel_id {
            parameters.push(("channelId".to_string(), channel_id.to_string()));
        }

        if let Some(is_reverse) = self.is_reverse {
            parameters.push(("isReverse".to_string(), is_reverse.to_string()));
        }

        parameters
    }
}

pub struct TransferProgress {
    pub(in crate::epg_station_api) current_bytes: u64,
    pub(in crate::epg_station_api) total_bytes: u64,
}

impl TransferProgress {
    pub fn current_bytes(&self) -> u64 {
        self.current_bytes
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}
