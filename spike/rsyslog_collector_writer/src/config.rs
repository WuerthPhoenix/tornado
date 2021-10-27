use clap::Parser;

#[derive(Debug, Parser)]
#[clap(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where Events are saved in JSON format
    #[clap(long, default_value = "./events")]
    pub json_events_path: String,

    /// How many times each event should be sent
    #[clap(long, default_value = "1000")]
    pub repeat_send: usize,

    /// How long to sleep after each message is sent, in milliseconds
    #[clap(long, default_value = "1000")]
    pub repeat_sleep_ms: u64,
}

#[derive(Debug, Parser)]
pub struct Conf {
    #[clap(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn build() -> Self {
        Conf::parse()
    }
}
