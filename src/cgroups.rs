use std::ffi::OsStr;
use std::fs;
use std::process::{Child, Command, Stdio};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CgroupsError {
    #[error("failed to retrieve cgroups")]
    Fetch(#[source] std::io::Error),
    #[error("no free cgroups left")]
    NoFree,
    #[error("failed to execute cgroups")]
    Execute(#[source] std::io::Error),
}

pub struct Cgroups {
    pub free: Vec<String>,
}

impl Cgroups {
    pub fn new() -> Result<Cgroups, CgroupsError> {
        Ok(Cgroups {
            free: get_cgroups().map_err(CgroupsError::Fetch)?,
        })
    }

    pub fn reserve(&mut self) -> Result<String, CgroupsError> {
        if self.free.len() == 0 {
            return Err(CgroupsError::NoFree);
        }

        Ok(self.free.swap_remove(0))
    }

    pub fn release(&mut self, cgroup: String) {
        self.free.push(cgroup);
    }

    pub fn execute(
        cgroup: &str,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<Child, CgroupsError> {
        let child = Command::new("cgexec")
            .arg("-g")
            .arg("memory,cpu:".to_string() + cgroup)
            .args(args)
            .stderr(Stdio::piped())
            .spawn()
            .map_err(CgroupsError::Execute)?;

        Ok(child)
    }
}

fn get_cgroups() -> Result<Vec<String>, std::io::Error> {
    Ok(fs::read_dir("/sys/fs/cgroup")?
        .filter_map(|dir| {
            dir.ok().and_then(|dir| {
                dir.path().file_name().and_then(|name| {
                    name.to_str().and_then(|x| {
                        if x.starts_with("workerd_") {
                            Some(x.to_owned())
                        } else {
                            None
                        }
                    })
                })
            })
        })
        .collect())
}
