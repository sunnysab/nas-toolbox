use super::TapeDevice;
use anyhow::{bail, Result};

/// Behaviour to handle End-Of-Tape.
#[repr(C)]
#[derive(Debug)]
pub enum EotModel {
    OneSetmark,
    TwoSetmarks,
    Many(u32),
}

mod ioctl_func {
    nix::ioctl_read!(get_eot_model, b'm', 8u8, u32);
    nix::ioctl_write_ptr!(set_eot_model, b'm', 8u8, u32);
}

impl TapeDevice {
    pub fn get_eot_model(&self) -> Result<EotModel> {
        let mut model = 0u32;

        unsafe {
            ioctl_func::get_eot_model(self.fd, &mut model)?;
        }
        let result = match model {
            1 => EotModel::OneSetmark,
            2 => EotModel::TwoSetmarks,
            _ => EotModel::Many(model),
        };
        Ok(result)
    }

    pub fn set_eot_model(&self, model: &EotModel) -> Result<()> {
        // From FreeBSD manual:
        // Set the EOT filemark model to argument and output the old and new models.  Typically this will be 2
        // filemarks, but some devices (typically QIC cartridge drives) can only write 1 filemark.
        // You may only choose a value of 1 or 2.
        let mut eot_model = match model {
            EotModel::OneSetmark => 1u32,
            EotModel::TwoSetmarks => 2u32,
            EotModel::Many(_) => {
                bail!("You may only choose a value of 1 or 2.");
            }
        };

        unsafe { ioctl_func::set_eot_model(self.fd, &mut eot_model)? };
        Ok(())
    }
}
