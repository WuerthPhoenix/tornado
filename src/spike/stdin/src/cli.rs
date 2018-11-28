use config::{Config, ConfigError, Environment, File, FileFormat};
use std::env;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(Debug, Serialize, Deserialize, StructOpt)]
#[structopt(name = "spike")]
pub struct Conf {

    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag.
    /// Activate debug mode
    #[structopt(short = "o", long = "value_one", default_value = "10000")]
    pub value_one: u64,

    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag.
    /// Activate debug mode
    #[structopt(short = "t", long = "value_two", default_value = "/tmp/tornado_spike_actix")]
    pub value_two: String,
}

impl Conf {
    pub fn new() -> Result<Self, ConfigError> {

        let defaults_opts = Conf::from_iter(vec![""]);
        let defaults_json = serde_json::to_string(&defaults_opts).unwrap_or_else(|err| panic!("{}", err));

        let args_opts = Conf::from_args();
        let args_json = serde_json::to_string(&args_opts).unwrap_or_else(|err| panic!("{}", err));

        let mut s = Config::new();

        //s.merge(File::from_str(&defaults_json, FileFormat::Json))?;

        s.merge(File::with_name("config/config"))?;

        //s.merge(File::from_str(&args_json, FileFormat::Json))?;

        let map: HashMap<String, String> = s.clone().try_into().unwrap();
        let list: Vec<String> = map.iter().map(|(k, v)| format!(r#"{}="{}""#, k, v)).collect();

        for entry in map {
            println!("map contains: [{}] - [{}]", entry.0, entry.1)
        }

        for entry in list {
            println!("list contains: [{}]", entry)
        }


        /*
        s.cache = serde_json::from_value(serde_json::to_value(opt)
            .unwrap_or_else(|err| panic!("{}", err)))
            .unwrap_or_else(|err| panic!("{}", err));
*/
//        s.merge(Environment::with_prefix("TORNADO_RSYSLOG"))?;

        s.try_into()
    }
}

pub fn print_cli() {
    println!("Print args:");
    for argument in env::args() {
        println!("{}", argument);
    }
}