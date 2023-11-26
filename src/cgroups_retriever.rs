use std::error::Error;

use serverless::cgroups;

fn main() -> Result<(), Box<dyn Error>> {
    let cgroups = cgroups::Cgroups::new()?;
    println!("{:?}", cgroups.free);

    Ok(())
}
