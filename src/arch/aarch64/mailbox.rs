use crate::arch::aarch64::framebuffer::FramebufferInfo;
use crate::arch::aarch64::mmio::{
    delay_us_sync, mmio_read, mmio_write, MBOX_READ, MBOX_STATUS, MBOX_WRITE,
};
use crate::prelude::*;
use core::cmp::{max, min};
use core::mem::size_of;
use core::ops::{Deref, DerefMut};
use core::ptr::slice_from_raw_parts;
use spin::Mutex;

/// This bit is set in the status register if there is no space to write into the mailbox
pub const MAIL_FULL: u32 = 0x80000000;

/// This bit is set in the status register if there is nothing to read from the mailbox
pub const MAIL_EMPTY: u32 = 0x40000000;

pub const MBOX_REQUEST: u32 = 0;

pub const MBOX_TAG_LAST: u32 = 0;

pub const MBOX_RESPONSE: u32 = 0x80000000;

#[repr(align(16), C)]
#[derive(Debug)]
struct MailboxMessage {
    size: u32,
    magic: u32,
    rest: [u32; 50],
}

static MAILBOX_LOCK: Mutex<()> = Mutex::new(());

#[link_section = ".dma"]
#[used]
static mut MAILBOX_MSG: MailboxMessage = MailboxMessage {
    size: 0,
    magic: 0,
    rest: [0; 50],
};

pub unsafe fn write_raw(data: u32) {
    while mmio_read(MBOX_STATUS) & MAIL_FULL != 0 {}
    mmio_write(MBOX_WRITE, data);
}

pub unsafe fn read_raw() -> u32 {
    let mut response = false;
    for _ in 0..1000 {
        if (mmio_read(MBOX_STATUS) & MAIL_EMPTY) == 0 {
            response = true;
            break;
        } else {
            delay_us_sync(100);
        }
    }
    if !response {
        panic!("No response from mailbox");
    }
    mmio_read(MBOX_READ)
}

pub unsafe fn call_raw(dst: *mut u8) {
    let mbox_addr = ((dst as usize as u32) & !0xF) | 8;
    write_raw(mbox_addr);
    while read_raw() != mbox_addr {}
}

// FIXME: HACK
pub(crate) unsafe fn _send_fb_property_tags() -> Result<FramebufferInfo, ()> {
    // Get reference to mailbox in device memory
    let _lock = MAILBOX_LOCK.lock();
    let mut mailbox = &mut MAILBOX_MSG;

    // Tag list header
    mailbox.size = 35 * 4;
    mailbox.magic = MBOX_REQUEST;

    mailbox.rest[0] = 0x48003;
    mailbox.rest[1] = 8;
    mailbox.rest[2] = 8;
    mailbox.rest[3] = 640;
    mailbox.rest[4] = 480;
    mailbox.rest[5] = 0x48004;
    mailbox.rest[6] = 8;
    mailbox.rest[7] = 8;
    mailbox.rest[8] = 640;
    mailbox.rest[9] = 480;
    mailbox.rest[10] = 0x48009;
    mailbox.rest[11] = 8;
    mailbox.rest[12] = 8;
    mailbox.rest[13] = 0;
    mailbox.rest[14] = 0;
    mailbox.rest[15] = 0x48005;
    mailbox.rest[16] = 4;
    mailbox.rest[17] = 4;
    mailbox.rest[18] = 32;
    mailbox.rest[19] = 0x48006;
    mailbox.rest[20] = 4;
    mailbox.rest[21] = 4;
    mailbox.rest[22] = 1;
    mailbox.rest[23] = 0x40001;
    mailbox.rest[24] = 8;
    mailbox.rest[25] = 8;
    mailbox.rest[26] = 4096;
    mailbox.rest[27] = 0;
    mailbox.rest[28] = 0x40008;
    mailbox.rest[29] = 4;
    mailbox.rest[30] = 4;
    mailbox.rest[31] = 0;

    mailbox.rest[32] = MBOX_TAG_LAST;

    // Send the tags
    call_raw(mailbox.deref_mut() as *mut MailboxMessage as *mut u8);

    if mailbox.magic != MBOX_RESPONSE {
        println!(
            "[EROR] No mailbox response (0x{:x} vs expected 0x{:x})",
            mailbox.magic, MBOX_RESPONSE
        );
        return Err(());
    }

    assert_eq!(mailbox.rest[18], 32);
    assert_ne!(mailbox.rest[26], 0);

    Ok(FramebufferInfo {
        width: mailbox.rest[3],
        height: mailbox.rest[4],
        virt_width: mailbox.rest[8],
        virt_height: mailbox.rest[9],
        pitch: mailbox.rest[31],
        depth: mailbox.rest[18],
        x_offset: mailbox.rest[13],
        y_offset: mailbox.rest[14],
        pointer: mailbox.rest[26] & 0x3fffffff,
        size: mailbox.rest[27],
    })
}

unsafe fn _send_property_tag(
    ident: u32,
    tag_capacity: u32,
    tag_data: &[u32],
) -> Result<TrimmedArray<u32, 26>, ()> {
    // Get reference to mailbox in device memory
    let _lock = MAILBOX_LOCK.lock();
    let mut mailbox = &mut MAILBOX_MSG;

    // Tag list header
    mailbox.size = tag_capacity + 6 * 4;
    mailbox.magic = MBOX_REQUEST;

    // First Tag
    mailbox.rest.fill(0);
    mailbox.rest[0] = ident;
    mailbox.rest[1] = tag_capacity;
    mailbox.rest[2] = 0;
    assert!(tag_data.len() <= 28);
    mailbox.rest[3..min(3 + tag_data.len(), 47)].copy_from_slice(tag_data);

    // Empty tag
    mailbox.rest[47] = MBOX_TAG_LAST;
    mailbox.rest[48] = 0;
    mailbox.rest[49] = 0;

    // Send the tags
    call_raw(mailbox.deref_mut() as *mut MailboxMessage as *mut u8);

    if mailbox.magic != MBOX_RESPONSE {
        println!(
            "[EROR] No mailbox response (0x{:x} vs expected 0x{:x})",
            mailbox.magic, MBOX_RESPONSE
        );
        return Err(());
    }

    // Find response tag
    let mut i = 0;
    loop {
        if i > 47 {
            return Err(());
        }
        let tag_ident = mailbox.rest[i];
        let tag_len = mailbox.rest[i + 1];
        if mailbox.rest[i] == ident {
            // Found response
            let mut tag_data = [0; 26];
            tag_data[..(tag_len / 4) as usize]
                .copy_from_slice(&mailbox.rest[i + 3..i + 3 + ((tag_len / 4) as usize)]);
            let tag_data = TrimmedArray::new(tag_data, (tag_len / 4) as usize);
            println!(
                "[DBUG] MB_RES: {:x}: {} bytes: {:?}",
                tag_ident,
                tag_len,
                tag_data.deref()
            );
            return Ok(tag_data);
        } else {
            // Not ours, skip
            println!(
                "[DBUG] MB_RES?: {:x}: {} bytes: {:?}",
                tag_ident,
                tag_len,
                &mailbox.rest[i + 3..i + 3 + (tag_len as usize)]
            );
            i += tag_len as usize + 3;
        }
    }
}

pub(crate) unsafe fn send_property_tag<REQ: Copy, RES: Copy>(
    ident: u32,
    req: REQ,
) -> Result<RES, ()> {
    let req = &*slice_from_raw_parts(&req as *const REQ as *const u32, size_of::<REQ>() / 4);
    let res = _send_property_tag(
        ident,
        max(size_of::<REQ>() as u32, size_of::<RES>() as u32),
        req,
    )?;
    let res = res.deref().as_ptr() as *const RES;
    Ok(*res)
}

pub(crate) unsafe fn send_property_tag_raw<REQ: Copy>(
    ident: u32,
    req: REQ,
    capacity: usize,
) -> Result<TrimmedArray<u32, 26>, ()> {
    let req = &*slice_from_raw_parts(&req as *const REQ as *const u32, size_of::<REQ>() / 4);
    let res = _send_property_tag(ident, max(size_of::<REQ>() as u32, capacity as u32), req)?;
    Ok(res)
}

pub struct TrimmedArray<T, const LEN: usize> {
    data: [T; LEN],
    len: usize,
}

impl<T, const LEN: usize> TrimmedArray<T, LEN> {
    pub fn new(data: [T; LEN], len: usize) -> Self {
        TrimmedArray { data, len }
    }
}

impl<T, const LEN: usize> Deref for TrimmedArray<T, LEN> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data[..self.len]
    }
}

impl<T, const LEN: usize> DerefMut for TrimmedArray<T, LEN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[..self.len]
    }
}
