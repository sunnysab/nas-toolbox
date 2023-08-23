mod device;

use crate::device::LocationBuilder;
use anyhow::Result;
use device::TapeDevice;

fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/nsa0")?;

    let model = device.get_eot_model();
    println!("{:?}", model);
    Ok(())
}
