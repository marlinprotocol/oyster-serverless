use std::error::Error;

use serverless::cgroups;

fn main() -> Result<(), Box<dyn Error>> {
    println!("{:?}", cgroups::get_cgroups()?);

    Ok(())
}
