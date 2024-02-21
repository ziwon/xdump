use std::{fs::File, io::BufReader};

use clap_serde_derive::{
    clap::{self, Parser},
    ClapSerde,
};
use rayon::ThreadPoolBuilder;
use serde_yaml;
use tokio::sync::mpsc;

mod xdump;
use crate::xdump::{capture::Capture, config::Config, scheduler::Scheduler, writer::Writer};
use ::xdump::xlog;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    input: Vec<std::path::PathBuf>,

    #[clap(short, long = "config", default_value = "config.yml")]
    config_path: std::path::PathBuf,

    #[clap(flatten)]
    pub config: <Config as ClapSerde>::Opt,
}

#[tokio::main]
async fn main() {
    let mut args = Args::parse();

    let config_file = File::options()
        .read(true)
        .write(false)
        .open(&args.config_path)
        .expect("No such file exist");

    let config: Config =
        match serde_yaml::from_reader::<_, <Config as ClapSerde>::Opt>(BufReader::new(config_file))
        {
            Ok(config) => Config::from(config).merge(&mut args.config),
            Err(err) => panic!("Error in configuration file:\n{}", err),
        };

    xlog!('I', format!("xdump started {:?}\r", config));

    ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();

    let (sender, receiver) = mpsc::channel(128);

    let scheduler = Scheduler::new(&config).expect("Failed to create Scheduler instance");
    let capture = Capture::new(&config, sender).expect("Failed to create Capture instance");
    let mut writer = Writer::new(&config, receiver).expect("Failed to create Writer instance");

    let s = tokio::spawn(async move {
        scheduler.run().await;
    });

    let c = tokio::spawn(async move {
        capture.start().await.expect("Capture task failed");
    });

    let w = tokio::spawn(async move {
        writer.write().await.expect("Writer task failed");
    });

    let _ = c.await;
    let _ = w.await;
    let _ = s.await;
}
