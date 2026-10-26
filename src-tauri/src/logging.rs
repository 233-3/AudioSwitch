//! 轻量文件日志。
//!
//! 需求：记录错误原因与切换过程、不记录敏感信息、有大小控制。
//! 实现：单个滚动文件 `app.log`，超过 512 KB 轮转为 `app.log.1` .. `app.log.3`，
//! 最旧的被丢弃，因此日志占用上限约 2 MB。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

const MAX_BYTES: u64 = 512 * 1024;
const KEEP_FILES: usize = 3;

struct Sink {
    file: Option<File>,
    written: u64,
}

pub struct FileLogger {
    dir: PathBuf,
    sink: Mutex<Sink>,
}

impl FileLogger {
    fn new(dir: PathBuf) -> Self {
        // 进程重启后延续已有文件大小，避免单次运行可以无限追加
        let existing = std::fs::metadata(dir.join("app.log"))
            .map(|meta| meta.len())
            .unwrap_or(0);
        Self {
            dir,
            sink: Mutex::new(Sink {
                file: None,
                written: existing,
            }),
        }
    }

    fn open(&self) -> Option<File> {
        let path = self.dir.join("app.log");
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("无法打开日志文件 {}：{err}", path.display());
                None
            }
        }
    }

    fn rotate(&self) {
        for index in (1..=KEEP_FILES).rev() {
            let from = if index == 1 {
                self.dir.join("app.log")
            } else {
                self.dir.join(format!("app.log.{}", index - 1))
            };
            let to = self.dir.join(format!("app.log.{index}"));
            if from.exists() {
                let _ = fs::rename(&from, &to);
            }
        }
        let _ = fs::remove_file(self.dir.join(format!("app.log.{}", KEEP_FILES + 1)));
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let line = format!(
            "[{}] [{}] {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            record.level(),
            record.args()
        );

        if cfg!(debug_assertions) {
            eprint!("{line}");
        }

        let mut sink = match self.sink.lock() {
            Ok(sink) => sink,
            Err(poisoned) => poisoned.into_inner(),
        };

        if sink.written + line.len() as u64 > MAX_BYTES {
            sink.file = None;
            self.rotate();
            sink.written = 0;
        }

        if sink.file.is_none() {
            sink.file = self.open();
        }

        if let Some(file) = sink.file.as_mut() {
            if file.write_all(line.as_bytes()).is_ok() {
                sink.written += line.len() as u64;
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut sink) = self.sink.lock() {
            if let Some(file) = sink.file.as_mut() {
                let _ = file.flush();
            }
        }
    }
}

/// 初始化全局日志。重复调用会被忽略。
pub fn init(log_dir: &Path) -> PathBuf {
    let _ = fs::create_dir_all(log_dir);
    let logger = Box::leak(Box::new(FileLogger::new(log_dir.to_path_buf())));

    if log::set_logger(logger).is_ok() {
        log::set_max_level(if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        });
    }

    log_dir.join("app.log")
}
