pub struct EncodeProgress {
    pub(in crate::encoder) current_secs: u64,
    pub(in crate::encoder) total_secs: u64,
}

impl EncodeProgress {
    pub fn current_secs(&self) -> u64 {
        self.current_secs
    }

    pub fn total_secs(&self) -> u64 {
        self.total_secs
    }
}
