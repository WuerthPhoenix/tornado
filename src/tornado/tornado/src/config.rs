use tornado_common_logger::LoggerConfig;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Io {
    /// The filesystem folder where the Rules are saved in json format.
    #[structopt(long, default_value="/etc/tornado/rules.d/")]
    pub rules_dir: String,

    /// The Unix Socket path where to listen for incoming events.
    #[structopt(long, default_value="/var/run/tornado/tornado.sock")]
    pub uds_path: String,
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
