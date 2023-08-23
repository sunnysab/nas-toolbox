mod device;

use crate::device::LocationBuilder;
use anyhow::Result;
use device::TapeDevice;

fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/nsa0")?;

    let location = LocationBuilder::new().block(4);
    let res = device.read_block_limit()?;

    println!("{:?}", res);
    Ok(())
}
