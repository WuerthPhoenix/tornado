use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {

    /// The filesystem folder where the Tornado configuration is saved.
    #[structopt(long, default_value = "/etc/tornado")]
    pub config_dir: String,

    /// The folder where the Rules are saved in json format.
    /// This folder is relative to the config_dir.
    #[structopt(long, default_value = "/rules.d/")]
    pub rules_dir: String,

    /// The Unix Socket path where to listen for incoming events.
    #[structopt(long, default_value = "/var/run/tornado/tornado.sock")]
    pub uds_path: String,

    /// The Unix Socket path where to listen for incoming snmptrapd events.
    #[structopt(long, default_value = "/var/run/tornado/tornado_snmptrapd.sock")]
    pub snmptrapd_uds_path: String,
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
