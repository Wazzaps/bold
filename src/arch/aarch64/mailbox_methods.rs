use crate::arch::aarch64::mailbox::send_property_tag;

pub fn get_clock_rate(clock_id: u32) -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetClockRateReq {
        clock_id: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetClockRateRes {
        clock_id: u32,
        rate: u32,
    }

    let res: GetClockRateRes =
        unsafe { send_property_tag(0x00030002, GetClockRateReq { clock_id })? };

    Ok(res.rate)
}

pub fn set_clock_rate(clock_id: u32, rate: u32, skip_setting_turbo: bool) -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetClockRateReq {
        clock_id: u32,
        rate: u32,
        skip_setting_turbo: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetClockRateRes {
        clock_id: u32,
        rate: u32,
    }

    let res: SetClockRateRes = unsafe {
        send_property_tag(
            0x00038002,
            SetClockRateReq {
                clock_id,
                rate,
                skip_setting_turbo: skip_setting_turbo as u32,
            },
        )?
    };

    Ok(res.rate)
}

/// Returns uptime in microseconds
pub fn get_stc() -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetStcReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetStcRes {
        time: u32,
        unused: u32,
    }

    let res: GetStcRes = unsafe { send_property_tag(0x0003000b, GetStcReq)? };

    Ok(res.time)
}
