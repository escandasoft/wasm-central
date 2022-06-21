pub mod watcher {

use std::path::Path;

#[derive(Debug)]
pub enum ChangedEntryStatus {
    DEPLOY, UNDEPLOY, RUNNING, UNDEPLOYED
}

#[derive(Debug)]
pub struct ChangedEntry {
    path: std::path::PathBuf,
    status: ChangedEntryStatus
}

#[derive(Debug)]
pub struct Watcher {
    pub dir: std::path::PathBuf
}

impl Watcher {
    pub fn new(p: &std::path::PathBuf) -> Self {
        Self { dir: p.clone() }
    }

    pub fn run(&self, callback: dyn FnOnce(&Path, &String) -> ()) -> () {
        let dir = std::fs::read_dir(&self.dir).unwrap();
        for result in dir {
            let file = result.expect("result needed");
            let path = file.path();
            if !path.is_dir() && path.ends_with(".zip") {
                let name = path.file_stem();
                let mut status_str = "deploy";
                for alternate_status in ["deploy", "undeploy", "running", "undeployed"] {
                    let rel_path = path.parent().unwrap().join(format!("{:?}.{}", name.expect("name"), &alternate_status));
                    if std::path::Path::new(&rel_path).exists() {
                        status_str = alternate_status;
                    }
                }
                let p = std::path::Path::new(&path);
                callback(&p, &String::from(status_str));
            }
        }
    }
 }
 
}