use crate::arch::aarch64::mailbox::{send_property_tag, send_property_tag_raw, TrimmedArray};
use crate::arch::aarch64::phymem::{PhyAddr, PhySlice};
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

pub fn get_framebuffer_phy_size() -> Result<(u32, u32), ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetFbPhySizeReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetFbPhySizeRes {
        width: u32,
        height: u32,
    }

    let res: GetFbPhySizeRes = unsafe { send_property_tag(0x00040003, GetFbPhySizeReq)? };

    Ok((res.width, res.height))
}

pub fn set_framebuffer_phy_size(width: u32, height: u32) -> Result<(u32, u32), ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbPhySizeReq {
        width: u32,
        height: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbPhySizeRes {
        width: u32,
        height: u32,
    }

    let res: SetFbPhySizeRes =
        unsafe { send_property_tag(0x00048003, SetFbPhySizeReq { width, height })? };

    Ok((res.width, res.height))
}

pub fn set_framebuffer_virt_size(width: u32, height: u32) -> Result<(u32, u32), ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbVirtSizeReq {
        width: u32,
        height: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbVirtSizeRes {
        width: u32,
        height: u32,
    }

    let res: SetFbVirtSizeRes =
        unsafe { send_property_tag(0x00048004, SetFbVirtSizeReq { width, height })? };

    Ok((res.width, res.height))
}

pub fn set_framebuffer_virt_offset(x: u32, y: u32) -> Result<(u32, u32), ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbVirtOffsetReq {
        x: u32,
        y: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbVirtOffsetRes {
        x: u32,
        y: u32,
    }

    let res: SetFbVirtOffsetRes =
        unsafe { send_property_tag(0x00048009, SetFbVirtOffsetReq { x, y })? };

    Ok((res.x, res.y))
}

pub fn set_framebuffer_depth(depth: u32) -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbDepthReq {
        depth: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbDepthRes {
        depth: u32,
    }

    let res: SetFbDepthRes = unsafe { send_property_tag(0x00048005, SetFbDepthReq { depth })? };

    Ok(res.depth)
}

pub fn set_framebuffer_pixel_order(order: u32) -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbPixelOrderReq {
        order: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SetFbPixelOrderRes {
        order: u32,
    }

    let res: SetFbPixelOrderRes =
        unsafe { send_property_tag(0x00040006, SetFbPixelOrderReq { order })? };

    Ok(res.order)
}

pub fn alloc_framebuffer(alignment: u32) -> Result<PhySlice, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct AllocFbReq {
        alignment: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct AllocFbRes {
        addr: u32,
        size: u32,
    }

    let res: AllocFbRes = unsafe { send_property_tag(0x00040001, AllocFbReq { alignment })? };

    Ok(PhySlice {
        base: PhyAddr((res.addr & 0x3fffffff) as usize),
        len: res.size as usize,
    })
}

pub fn free_framebuffer() -> Result<(), ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct FreeFbReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct FreeFbRes;

    let _res: FreeFbRes = unsafe { send_property_tag(0x00048001, FreeFbReq)? };

    Ok(())
}

pub fn get_framebuffer_pitch() -> Result<u32, ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetFbPitchReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetFbPitchRes {
        pitch: u32,
    }

    let res: GetFbPitchRes = unsafe { send_property_tag(0x00040008, GetFbPitchReq)? };

    Ok(res.pitch)
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

/// Returns MAC address
pub fn get_nic_mac() -> Result<[u8; 6], ()> {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetNicMacReq;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct GetNicMacRes {
        mac: [u8; 6],
    }

    let res: GetNicMacRes = unsafe { send_property_tag(0x00010003, GetNicMacReq)? };

    Ok(res.mac)
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
