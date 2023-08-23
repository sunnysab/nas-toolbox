mod device;

use device::TapeDevice;
use anyhow::Result;

pub struct DriverLimit {

}


fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/sa0")?;
    let status = device.status_ex()?;
    Ok(())
}
