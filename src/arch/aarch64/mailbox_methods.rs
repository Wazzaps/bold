use crate::arch::aarch64::mailbox::{send_property_tag, send_property_tag_raw, TrimmedArray};
use crate::console::Freq;
use core::ops::Deref;
use core::ptr::slice_from_raw_parts;

pub fn get_clock_rate(clock_id: u32) -> Result<Freq, ()> {
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

    Ok(Freq(res.rate as u64))
}

#[allow(dead_code)]
pub fn set_clock_rate(clock_id: u32, rate: u32, skip_setting_turbo: bool) -> Result<Freq, ()> {
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

    Ok(Freq(res.rate as u64))
}

/// Returns uptime in microseconds
#[allow(dead_code)]
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

/// Returns kernel args
pub fn get_kernel_args() -> Result<TrimmedArray<u8, 104>, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetKernelArgsReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetKernelArgsRes {
        data: [u8; 96],
    }

    let res = unsafe { send_property_tag_raw(0x00050001, GetKernelArgsReq, 104)? };
    let transmuted =
        unsafe { &*slice_from_raw_parts(res.deref().as_ptr() as *const u8, res.len() * 4) };

    let mut new_res_data = [0u8; 104];
    new_res_data[0..transmuted.len()].copy_from_slice(transmuted);

    Ok(TrimmedArray::new(new_res_data, transmuted.len()))
}
