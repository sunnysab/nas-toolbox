mod device;

use crate::device::LocationBuilder;
use anyhow::Result;
use device::TapeDevice;

fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/nsa0")?;

    let err = device.get_last_error()?;
    println!("{:?}", err);
    Ok(())
}
