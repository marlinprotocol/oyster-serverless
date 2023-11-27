use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CgroupsError {
    #[error("failed to retrieve cgroups")]
    Fetch(#[source] std::io::Error),
}

pub struct Cgroups {
    pub free: Vec<String>,
    pub used: Vec<String>,
}

impl Cgroups {
    pub fn new() -> Result<Cgroups, CgroupsError> {
        Ok(Cgroups {
            used: Vec::new(),
            free: get_cgroups().map_err(CgroupsError::Fetch)?,
        })
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
