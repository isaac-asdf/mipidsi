use display_interface::{DataFormat, WriteOnlyDataCommand};
use embedded_graphics_core::{pixelcolor::Rgb565, prelude::IntoStorage};
use embedded_hal::{delay::DelayNs, digital::OutputPin};

use crate::{
    dcs::{BitsPerPixel, Dcs, PixelFormat, SetAddressMode, SoftReset, WriteMemoryStart},
    error::{Error, InitError},
    options::ModelOptions,
};

use crate::dcs::{EnterNormalMode, ExitSleepMode, SetDisplayOn, SetInvertMode, SetPixelFormat};

use super::Model;

/// ILI9488 display in Rgb565 color mode.
pub struct ILI9488Rgb565;

impl Model for ILI9488Rgb565 {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = (320, 480);

    fn init<RST, DELAY, DI>(
        &mut self,
        dcs: &mut Dcs<DI>,
        delay: &mut DELAY,
        options: &ModelOptions,
        rst: &mut Option<RST>,
    ) -> Result<SetAddressMode, InitError<RST::Error>>
    where
        RST: OutputPin,
        DELAY: DelayNs,
        DI: WriteOnlyDataCommand,
    {
        match rst {
            Some(ref mut rst) => self.hard_reset(rst, delay)?,
            None => dcs.write_command(SoftReset)?,
        }
        delay.delay_us(120_000);

        let pf = PixelFormat::with_all(BitsPerPixel::from_rgb_color::<Self::ColorFormat>());
        Ok(init_common(dcs, delay, options, pf)?)
    }

    fn write_pixels<DI, I>(&mut self, dcs: &mut Dcs<DI>, colors: I) -> Result<(), Error>
    where
        DI: WriteOnlyDataCommand,
        I: IntoIterator<Item = Self::ColorFormat>,
    {
        dcs.write_command(WriteMemoryStart)?;
        let mut iter = colors.into_iter().map(|c| c.into_storage());

        let buf = DataFormat::U16BEIter(&mut iter);
        dcs.di.send_data(buf)
    }
}

// common init for all color format models
fn init_common<DELAY, DI>(
    dcs: &mut Dcs<DI>,
    delay: &mut DELAY,
    options: &ModelOptions,
    pixel_format: PixelFormat,
) -> Result<SetAddressMode, Error>
where
    DELAY: DelayNs,
    DI: WriteOnlyDataCommand,
{
    let madctl = SetAddressMode::from(options);
    dcs.write_command(ExitSleepMode)?; // turn off sleep
    dcs.write_command(SetPixelFormat::new(pixel_format))?; // pixel format
    dcs.write_command(madctl)?; // left -> right, bottom -> top RGB
    dcs.write_command(SetInvertMode::new(options.invert_colors))?;
    dcs.write_raw(0xc5, &[0x00, 0x1e, 0x80, 0xb1])?; // vcom control
    dcs.write_raw(0xb1, &[0xb0])?; // frame rate

    // optional gamma setup
    dcs.write_raw(
        0xe0,
        &[
            0x0, 0x13, 0x18, 0x04, 0x0F, 0x06, 0x3a, 0x56, 0x4d, 0x03, 0x0a, 0x06, 0x30, 0x3e, 0x0f,
        ],
    )?;
    dcs.write_raw(
        0xe1,
        &[
            0x0, 0x13, 0x18, 0x01, 0x11, 0x06, 0x38, 0x34, 0x4d, 0x06, 0x0d, 0x0b, 0x31, 0x37, 0x0f,
        ],
    )?;

    // dcs.write_raw(0x3a, &[0x55])?; // set 16-bit pixel display

    // NOTE: manually setting memory access data control, ignoring passed in
    let _l2r_u2d = 0x22; // blank
    let _d2u_l2r = 0x62; // blank
    let r2l_d2u = 0x42; // worked
    let u2d_r2l = 0x02; // looked same
    dcs.write_raw(0xB6, &[0b0000_0000, r2l_d2u])?; // L2R_U2D

    dcs.write_command(EnterNormalMode)?; // turn to normal mode
    dcs.write_command(SetDisplayOn)?; // turn on display

    // DISPON requires some time otherwise we risk SPI data issues
    delay.delay_us(120_000);

    Ok(madctl)
}

#[cfg(test)]
mod tests {
    use crate::dcs::{self, DcsCommand};

    #[test]
    fn cm_struct() {
        let cm1 = [0x2a, 0x00, 0x00, 0x00, 0x05];
        let cm2 = [0x2b, 0x00, 0x00, 0x00, 0x05];
        let cm3 = [0x2c, 0xff, 0xff, 0xff, 0xff];
        let sx = 0;
        let sy = 0;
        let ex = 5;
        let ey = 5;
        let res1 = dcs::SetColumnAddress::new(sx, ex);
        let res2 = dcs::SetPageAddress::new(sy, ey);
        let mut param_bytes: [u8; 16] = [0; 16];
        let n = res1.fill_params_buf(&mut param_bytes).unwrap();
        let ins = res1.instruction();

        assert_eq!(cm1[0], ins);
        assert_eq!(cm1[1..4], param_bytes[0..3]);
    }
}
