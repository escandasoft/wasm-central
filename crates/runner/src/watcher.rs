use std::path::{Path, PathBuf};

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

impl DirectoryWatcher {
    pub fn new(p: PathBuf) -> Self {
        Self { dir: p }
    }

    pub fn run(&self) -> Vec<WatcherEntry> {
        let dir = std::fs::read_dir(&self.dir).unwrap();
        let mut dropped_files = vec![];
        for result in dir {
            let file = result.expect("result needed");
            println!(
                "!! found file at {}",
                String::from(file.file_name().to_str().unwrap())
            );
            let path = file.path();
            let ext = path.extension();
            if ext.is_some() && ext.unwrap().eq("zip") {
                let name = path.file_stem();
                let mut status_str = "deploy";
                for alternate_status in ["deploy", "undeploy", "running", "undeployed"] {
                    let part_path = format!("{:?}.{}", name.expect("name"), &alternate_status);
                    let rel_path = path.parent().unwrap().join(part_path);
                    if Path::new(&rel_path).exists() {
                        status_str = alternate_status;
                    }
                }
                let p = Path::new(&path);
                let pbuf = p.to_path_buf();
                let next_status = String::from(status_str);
                println!(
                    "!! added dropped file to {} with {} status",
                    pbuf.display(),
                    next_status
                );
                dropped_files.push(WatcherEntry {
                    path: pbuf,
                    next_status: next_status,
                });
            }
        }
        return dropped_files;
    }
}
