use clap::Clap;

#[derive(Debug, Clap)]
#[clap(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado configuration is saved
    #[clap(long, default_value = "/etc/tornado/tornado_events_generator")]
    pub config_dir: String,

    /// The filesystem folder where Events are saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[clap(long, default_value = "/events")]
    pub events_dir: String,

    /// The Tornado TCP address where outgoing events will be written
    #[clap(long, default_value = "127.0.0.1:4747")]
    pub tornado_tcp_address: String,

    /// How many times each event should be sent
    #[clap(long, default_value = "1000")]
    pub repeat_send: usize,

    /// How long to sleep after each event is sent, in milliseconds
    #[clap(long, default_value = "2000")]
    pub repeat_sleep_ms: u64,
}

#[derive(Debug, Clap)]
pub struct Conf {
    #[clap(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn build() -> Self {
        Conf::parse()
    }
}
