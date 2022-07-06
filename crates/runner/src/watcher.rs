use std::fs;
use std::path::{Path, PathBuf};
use libc::stat;

#[derive(Debug)]
pub enum ChangedEntryStatus {
    DEPLOY,
    UNDEPLOY,
    RUNNING,
    UNDEPLOYED,
}

#[derive(Debug)]
pub struct WatcherEntry {
    pub path: PathBuf,
    pub next_status: String,
}

#[derive(Debug)]
pub struct DirectoryWatcher {
    pub dir: PathBuf,
}

const ALTERNATE_STATES: [&str;5] = ["deploy", "undeploy", "running", "undeployed", "redeploy"];

impl DirectoryWatcher {
    pub fn new(p: PathBuf) -> Self {
        Self { dir: p }
    }

    pub fn remove_next_states(&self) {
        for result in std::fs::read_dir(&self.dir).expect("read_dir fs") {
            let file = result.expect("DirEntry");
            for state in ALTERNATE_STATES {
                let path_buf = file.path();
                let tentative_name = path_buf.file_stem().unwrap().to_str().unwrap().to_owned();
                let state_path = self.dir.join(format!("{}.{}", tentative_name, state));
                if state_path.exists() {
                    if let Err(err) = fs::remove_file(state_path.clone()) {
                        eprintln!("Cannot remove orphan state at {:?}", state_path);
                    }
                }
            }
        }
    }

    pub fn run(&self) -> Vec<WatcherEntry> {
        let dir = std::fs::read_dir(&self.dir).unwrap();
        let mut dropped_files = vec![];
        for result in dir {
            let file = result.expect("result needed");
            let path = file.path();
            let ext = path.extension();
            if ext.is_some() && ext.unwrap().eq("wasm") {
                let name = path.file_stem();
                let mut status_str = "deploy";
                for alternate_status in ALTERNATE_STATES {
                    let part_path = format!("{}.{}", name.unwrap().to_str().to_owned().unwrap(), &alternate_status);
                    let rel_path = path.parent().unwrap().join(part_path);
                    if rel_path.exists() {
                        status_str = alternate_status;
                    }
                }
                let p = Path::new(&path);
                let pbuf = p.to_path_buf();
                let next_status = String::from(status_str);
                dropped_files.push(WatcherEntry {
                    path: pbuf,
                    next_status,
                });
            }
        }
        dropped_files
    }
}
