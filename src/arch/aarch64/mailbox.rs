use crate::arch::aarch64::mmio::{mmio_read, mmio_write, MBOX_READ, MBOX_STATUS, MBOX_WRITE};
use crate::arch::aarch64::tty::dump_hex;
use crate::println;
use core::mem::{size_of, ManuallyDrop};

/// This bit is set in the status register if there is no space to write into the mailbox
pub const MAIL_FULL: u32 = 0x80000000;

/// This bit is set in the status register if there is nothing to read from the mailbox
pub const MAIL_EMPTY: u32 = 0x40000000;

pub unsafe fn write_raw(data: u32) {
    while mmio_read(MBOX_STATUS) & MAIL_FULL != 0 {}
    mmio_write(MBOX_WRITE, data);
}

pub unsafe fn read_raw(channel: u32) -> u32 {
    loop {
        while (mmio_read(MBOX_STATUS) & MAIL_EMPTY) == 0 {}
        let val = mmio_read(MBOX_READ);
        if val & 0xF == channel {
            return val & !0xF;
        }
    }
}

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

    impl MailboxRequest for GetClockRateReq {
        const ID: u32 = 0x00030002;
        type RESPONSE = GetClockRateRes;
    }

    let res = unsafe {
        MailboxMessage::send((MailboxTag::new(GetClockRateReq { clock_id }),))?
            .0
            .data()?
    };
    Ok(res.rate)
}

pub fn set_clock_rate(clock_id: u32, rate: u32, skip_setting_turbo: bool) -> Result<u32, ()> {
    // 2, 3000000, 0
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

    impl MailboxRequest for SetClockRateReq {
        const ID: u32 = 0x00038002;
        type RESPONSE = SetClockRateRes;
    }

    let res = unsafe {
        MailboxMessage::send((MailboxTag::new(SetClockRateReq {
            clock_id,
            rate,
            skip_setting_turbo: skip_setting_turbo as u32,
        }),))?
        .0
        .data()?
    };

    Ok(res.rate)
}

#[repr(align(4), C)]
pub struct MailboxTag<REQ: MailboxRequest> {
    id: u32,
    len: u32,
    code: u32,
    data: MailboxTagData<REQ, REQ::RESPONSE>,
}

impl<REQ: MailboxRequest> MailboxTag<REQ> {
    pub fn new(data: REQ) -> Self {
        MailboxTag {
            id: REQ::ID,
            len: unsafe { size_of::<MailboxTagData<REQ, REQ::RESPONSE>>() as u32 },
            code: 0,
            data: MailboxTagData {
                req: ManuallyDrop::new(data),
            },
        }
    }

    pub unsafe fn data(self) -> Result<REQ::RESPONSE, ()> {
        // if self.code & 0x80000000 == 0 {
        //     Err(())
        // } else {
        //     Ok(*self.data.res)
        // }
        Ok(*self.data.res)
    }
}

#[repr(align(4), C)]
union MailboxTagData<REQ, RES> {
    req: ManuallyDrop<REQ>,
    res: ManuallyDrop<RES>,
}

#[repr(align(16), C)]
pub struct MailboxMessage<MSG> {
    len: u32,
    code: u32,
    data: MSG,
    end_tag: u32,
}

impl<MSG> MailboxMessage<MSG> {
    pub unsafe fn send(data: MSG) -> Result<MSG, ()> {
        let mut msg = MailboxMessage {
            len: (size_of::<MSG>() + 12) as u32,
            code: 0,
            data,
            end_tag: 0,
        };

        write_raw(((&mut msg as *mut MailboxMessage<MSG> as *mut u8 as usize as u32) & !0xF) | 8);

        // match msg.code {
        //     0x80000000 => Ok(msg.data),
        //     code => {
        //         panic!("code was {:x}", msg.code);
        //         Err(())
        //     },
        // }

        Ok(msg.data)
    }
}

pub trait MailboxRequest {
    const ID: u32;
    type RESPONSE: Copy;
}
