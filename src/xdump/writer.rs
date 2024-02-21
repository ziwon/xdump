use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use pcap_file::{
    pcap::{PcapHeader, PcapWriter, RawPcapPacket},
    Endianness, PcapError,
};
use rayon::prelude::*;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

use super::{config::Config, global::check_capture_state};
use xdump::xlog;

#[derive(Error, Debug)]
pub enum WriterError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Pcap error: {0}")]
    Pcap(#[from] PcapError),
}

pub struct Writer {
    data_home: PathBuf,
    prefix: String,
    is_capturing: bool,
    receiver: Receiver<Vec<u8>>,
}

impl Writer {
    pub fn new(config: &Config, receiver: Receiver<Vec<u8>>) -> Result<Self, WriterError> {
        let data_home = Self::check_path(&config.data_home)?;
        xlog!('I', format!("[Data Home]: {:?}", data_home));
        Ok(Writer {
            data_home,
            prefix: config.file_prefix.to_string(),
            is_capturing: false,
            receiver,
        })
    }

    pub async fn write(&mut self) -> Result<(), WriterError> {
        let mut pcap_writer: Option<PcapWriter<BufWriter<File>>> = None;

        loop {
            let current_state = check_capture_state();
            if current_state && !self.is_capturing {
                self.is_capturing = true;
                let datetime = chrono::Local::now();
                let file_name = format!("{}-{}.pcap", self.prefix, datetime.format("%Y%m%d"));
                let file_path = self.data_home.join(file_name);
                xlog!('I', format!("Creating new pcap file: {:?}", file_path));

                let file = File::create(&file_path)?;
                let writer = BufWriter::new(file);
                let header = PcapHeader {
                    endianness: if cfg!(target_endian = "big") {
                        Endianness::Big
                    } else {
                        Endianness::Little
                    },
                    ..PcapHeader::default()
                };

                pcap_writer = Some(PcapWriter::with_header(writer, header)?);
            } else if !current_state && self.is_capturing {
                pcap_writer = None; // Let PcapWriter be dropped
                self.is_capturing = false;
            }

            if self.is_capturing {
                if let Some(packet) = self.receiver.recv().await {
                    if let Some(pcap) = pcap_writer.as_mut() {
                        let now = SystemTime::now();
                        let ts = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                        let ts_sec = ts.as_secs() as u32;
                        let ts_frac = ts.subsec_micros();

                        let data: Vec<u8> = packet.par_iter().map(|&byte| byte).collect();

                        let raw_packet = RawPcapPacket {
                            ts_sec,
                            ts_frac,
                            incl_len: data.len() as u32,
                            orig_len: data.len() as u32,
                            data: data.into(),
                        };

                        pcap.write_raw_packet(&raw_packet)?;
                    }
                }
            } else {
                // Waiting..
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    fn check_path(path_name: &str) -> Result<PathBuf, WriterError> {
        let path = Path::new(path_name);

        fs::metadata(path)
            .map(|_| path.to_path_buf())
            .map_err(|_| WriterError::InvalidPath(path_name.into()))
    }
}
