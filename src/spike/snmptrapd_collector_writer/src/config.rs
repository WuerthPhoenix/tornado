use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where Events are saved in JSON format
    #[structopt(long, default_value = "./events")]
    pub json_events_path: String,

    /// The TCP address where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "0.0.0.0:4748")]
    pub snmptrapd_tornado_tpc_address: String,

    /// How many times each event should be sent
    #[structopt(long, default_value = "1000")]
    pub repeat_send: usize,

    /// How long to sleep after each message is sent, in milliseconds
    #[structopt(long, default_value = "1000")]
    pub repeat_sleep_ms: u64,
}

#[derive(Debug, StructOpt)]
pub struct Conf {
    #[structopt(flatten)]
    pub logger: LoggerConfig,

    #[structopt(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}
