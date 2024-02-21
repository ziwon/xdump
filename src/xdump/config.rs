use clap_serde_derive::{serde::Serialize, ClapSerde};

#[derive(ClapSerde, Serialize, Debug)]
pub struct Config {
    /// Network interface to capture packets from
    #[clap(short = 'i', long = "interface")]
    pub interface: String,

    /// Data path to save captured packets
    #[clap(short = 'd', long = "data-home")]
    pub data_home: String,

    /// Start time to capture
    #[clap(short = 's', long = "start-time")]
    pub start_time: String,

    /// End time to capture
    #[clap(short = 'e', long = "end-time")]
    pub end_time: String,

    /// Excluded ports from capturing
    #[clap(short = 'x', long = "excluded-ports")]
    pub excluded_ports: Vec<u16>,

    /// Prefix for captured file
    #[clap(short = 'p', long = "file_prefix")]
    pub file_prefix: String,
}
