#[derive(Debug)]
pub struct ChannelDirectory {
    pub prefix: String,
    pub channel: String,
    pub raw: String,
    pub jsonl: String,
}

impl ChannelDirectory {
    pub fn new(channel: &str, base: &str) -> Self {
        Self {
            prefix: format!("{base}/subtitle"),
            channel: String::from(channel),
            raw: String::from("raw"),
            jsonl: String::from("jsonl"),
        }
    }

    pub fn manifest_filepath(&self) -> String {
        format!("{}/{}/manifest.json", self.prefix, self.channel)
    }

    pub fn raw_output_dir(&self) -> String {
        format!("subtitle:{}/{}/{}", self.prefix, self.channel, self.raw)
    }

    pub fn jsonl_output_dir(&self) -> String {
        format!("{}/{}/{}", self.prefix, self.channel, self.jsonl)
    }
}
