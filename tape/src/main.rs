mod device;

use crate::device::LocationBuilder;
use anyhow::Result;
use device::TapeDevice;

pub struct DriverLimit {}

fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/nsa0")?;

    let location = LocationBuilder::new().block(4);
    let res = device.locate_to(&location)?;

    println!("{res}");
    Ok(())
}
