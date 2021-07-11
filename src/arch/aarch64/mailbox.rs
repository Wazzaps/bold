use crate::arch::aarch64::mmio::{
    delay_us_sync, mmio_read, mmio_write, MBOX_READ, MBOX_STATUS, MBOX_WRITE,
};
use crate::println;
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
    rest: [u32; 34],
}

static MAILBOX_LOCK: Mutex<()> = Mutex::new(());

#[link_section = ".dma"]
#[used]
static mut MAILBOX_MSG: MailboxMessage = MailboxMessage {
    size: 0,
    magic: 0,
    rest: [0; 34],
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
    mailbox.rest[3..min(3 + tag_data.len(), 31)].copy_from_slice(tag_data);

    // Empty tag
    mailbox.rest[31] = MBOX_TAG_LAST;
    mailbox.rest[32] = 0;
    mailbox.rest[33] = 0;

    // Send the tags
    call_raw(mailbox.deref_mut() as *mut MailboxMessage as *mut u8);

    if mailbox.magic != MBOX_RESPONSE {
        println!("[EROR] Wrong mailbox response");
        return Err(());
    }

    // Find response tag
    let mut i = 0;
    loop {
        if i > 31 {
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
