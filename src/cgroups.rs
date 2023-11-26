use std::fs;

pub struct Cgroups {
    pub free: Vec<String>,
    pub used: Vec<String>,
}

impl Cgroups {
    pub fn new() -> Result<Cgroups, std::io::Error> {
        Ok(Cgroups {
            free: get_cgroups()?,
            used: Vec::new(),
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
