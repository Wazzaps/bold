use crate::arch::aarch64::mmio::{
    delay, delay_us_sync, mmio_read, mmio_write, GPFSEL4, GPFSEL5, GPHEN1, GPPUD, GPPUDCLK1,
};
use crate::{print, println};

pub struct Sdhc {
    sd_scr: [u32; 2],
    #[allow(dead_code)]
    sd_ocr: u32, // TODO: remove?
    sd_rca: u32,
    sd_hv: u32,
}

mod consts {
    #![allow(dead_code)]
    use crate::arch::aarch64::mmio::MMIO_BASE;

    pub const EMMC_ARG2: u32 = MMIO_BASE + 0x00300000;
    pub const EMMC_BLKSIZECNT: u32 = MMIO_BASE + 0x00300004;
    pub const EMMC_ARG1: u32 = MMIO_BASE + 0x00300008;
    pub const EMMC_CMDTM: u32 = MMIO_BASE + 0x0030000C;
    pub const EMMC_RESP0: u32 = MMIO_BASE + 0x00300010;
    pub const EMMC_RESP1: u32 = MMIO_BASE + 0x00300014;
    pub const EMMC_RESP2: u32 = MMIO_BASE + 0x00300018;
    pub const EMMC_RESP3: u32 = MMIO_BASE + 0x0030001C;
    pub const EMMC_DATA: u32 = MMIO_BASE + 0x00300020;
    pub const EMMC_STATUS: u32 = MMIO_BASE + 0x00300024;
    pub const EMMC_CONTROL0: u32 = MMIO_BASE + 0x00300028;
    pub const EMMC_CONTROL1: u32 = MMIO_BASE + 0x0030002C;
    pub const EMMC_INTERRUPT: u32 = MMIO_BASE + 0x00300030;
    pub const EMMC_INT_MASK: u32 = MMIO_BASE + 0x00300034;
    pub const EMMC_INT_EN: u32 = MMIO_BASE + 0x00300038;
    pub const EMMC_CONTROL2: u32 = MMIO_BASE + 0x0030003C;
    pub const EMMC_SLOTISR_VER: u32 = MMIO_BASE + 0x003000FC;

    // command flags
    pub const CMD_NEED_APP: u32 = 0x80000000;
    pub const CMD_RSPNS_48: u32 = 0x00020000;
    pub const CMD_ERRORS_MASK: u32 = 0xfff9c004;
    pub const CMD_RCA_MASK: u32 = 0xffff0000;

    // COMMANDs
    pub const CMD_GO_IDLE: u32 = 0x00000000;
    pub const CMD_ALL_SEND_CID: u32 = 0x02010000;
    pub const CMD_SEND_REL_ADDR: u32 = 0x03020000;
    pub const CMD_CARD_SELECT: u32 = 0x07030000;
    pub const CMD_SEND_IF_COND: u32 = 0x08020000;
    pub const CMD_STOP_TRANS: u32 = 0x0C030000;
    pub const CMD_READ_SINGLE: u32 = 0x11220010;
    pub const CMD_READ_MULTI: u32 = 0x12220032;
    pub const CMD_WRITE_SINGLE: u32 = 0x18220000;
    pub const CMD_WRITE_MULTI: u32 = 0x19220022;
    pub const CMD_SET_BLOCKCNT: u32 = 0x17020000;
    pub const CMD_APP_CMD: u32 = 0x37000000;
    pub const CMD_SET_BUS_WIDTH: u32 = 0x06020000 | CMD_NEED_APP;
    pub const CMD_SEND_OP_COND: u32 = 0x29020000 | CMD_NEED_APP;
    pub const CMD_SEND_SCR: u32 = 0x33220010 | CMD_NEED_APP;

    // STATUS register settings
    pub const SR_READ_AVAILABLE: u32 = 0x00000800;
    pub const SR_WRITE_AVAILABLE: u32 = 0x00000400;
    pub const SR_DAT_INHIBIT: u32 = 0x00000002;
    pub const SR_CMD_INHIBIT: u32 = 0x00000001;
    pub const SR_APP_CMD: u32 = 0x00000020;

    // INTERRUPT register settings
    pub const INT_DATA_TIMEOUT: u32 = 0x00100000;
    pub const INT_CMD_TIMEOUT: u32 = 0x00010000;
    pub const INT_READ_RDY: u32 = 0x00000020;
    pub const INT_WRITE_RDY: u32 = 0x00000010;
    pub const INT_DATA_DONE: u32 = 0x00000002;
    pub const INT_CMD_DONE: u32 = 0x00000001;

    pub const INT_ERROR_MASK: u32 = 0x017E8000;

    // CONTROL register settings
    pub const C0_SPI_MODE_EN: u32 = 0x00100000;
    pub const C0_HCTL_HS_EN: u32 = 0x00000004;
    pub const C0_HCTL_DWITDH: u32 = 0x00000002;

    pub const C1_SRST_DATA: u32 = 0x04000000;
    pub const C1_SRST_CMD: u32 = 0x02000000;
    pub const C1_SRST_HC: u32 = 0x01000000;
    pub const C1_TOUNIT_DIS: u32 = 0x000f0000;
    pub const C1_TOUNIT_MAX: u32 = 0x000e0000;
    pub const C1_CLK_GENSEL: u32 = 0x00000020;
    pub const C1_CLK_EN: u32 = 0x00000004;
    pub const C1_CLK_STABLE: u32 = 0x00000002;
    pub const C1_CLK_INTLEN: u32 = 0x00000001;

    // SLOTISR_VER values
    pub const HOST_SPEC_NUM: u32 = 0x00ff0000;
    pub const HOST_SPEC_NUM_SHIFT: u32 = 16;
    pub const HOST_SPEC_V3: u32 = 2;
    pub const HOST_SPEC_V2: u32 = 1;
    pub const HOST_SPEC_V1: u32 = 0;

    // SCR flags
    pub const SCR_SD_BUS_WIDTH_4: u32 = 0x00000400;
    pub const SCR_SUPP_SET_BLKCNT: u32 = 0x02000000;
    // added by my driver
    pub const SCR_SUPP_CCS: u32 = 0x00000001;

    pub const ACMD41_VOLTAGE: u32 = 0x00ff8000;
    pub const ACMD41_CMD_COMPLETE: u32 = 0x80000000;
    pub const ACMD41_CMD_CCS: u32 = 0x40000000;
    pub const ACMD41_ARG_HC: u32 = 0x51ff8000;
}

use consts::*;

pub struct SdhcCmdError(pub u32);

impl From<SdhcCmdError> for () {
    fn from(e: SdhcCmdError) -> Self {
        println!("[WARN] ERROR: Cmd returned error 0x{:x}", e.0);
    }
}

impl Sdhc {
    pub unsafe fn init() -> Result<Self, ()> {
        let mut sdhc = Self {
            sd_scr: [0, 0],
            sd_ocr: 0,
            sd_rca: 0,
            sd_hv: 0,
        };

        // GPIO_CD
        mmio_write(GPFSEL4, mmio_read(GPFSEL4) & !(7 << (7 * 3)));
        mmio_write(GPPUD, 2);
        delay(150);
        mmio_write(GPPUDCLK1, 1 << 15);
        delay(150);
        mmio_write(GPPUD, 0);
        mmio_write(GPPUDCLK1, 0);
        mmio_write(GPHEN1, mmio_read(GPHEN1) | 1 << 15);

        // GPIO_CLK, GPIO_CMD
        mmio_write(
            GPFSEL4,
            mmio_read(GPFSEL4) | (7 << (8 * 3)) | (7 << (9 * 3)),
        );
        mmio_write(GPPUD, 2);
        delay(150);
        mmio_write(GPPUDCLK1, (1 << 16) | (1 << 17));
        delay(150);
        mmio_write(GPPUD, 0);
        mmio_write(GPPUDCLK1, 0);

        // GPIO_DAT0, GPIO_DAT1, GPIO_DAT2, GPIO_DAT3
        mmio_write(
            GPFSEL5,
            mmio_read(GPFSEL5) | 7 | (7 << 3) | (7 << (2 * 3)) | (7 << (3 * 3)),
        );
        mmio_write(GPPUD, 2);
        delay(150);
        mmio_write(GPPUDCLK1, (1 << 18) | (1 << 19) | (1 << 20) | (1 << 21));
        delay(150);
        mmio_write(GPPUD, 0);
        mmio_write(GPPUDCLK1, 0);

        sdhc.sd_hv = (mmio_read(EMMC_SLOTISR_VER) & HOST_SPEC_NUM) >> HOST_SPEC_NUM_SHIFT;
        println!("[INFO] EMMC GPIO set up");

        // Reset the card
        mmio_write(EMMC_CONTROL0, 0);
        mmio_write(EMMC_CONTROL1, mmio_read(EMMC_CONTROL1) | C1_SRST_HC);
        let mut successful = false;
        for _ in 0..10_000 {
            if mmio_read(EMMC_CONTROL1) & C1_SRST_HC == 0 {
                successful = true;
                break;
            }
            delay_us_sync(10);
        }
        if successful {
            println!("[INFO] EMMC: reset OK");
        } else {
            println!("[WARN] EMMC: reset failed");
            return Err(());
        }
        mmio_write(
            EMMC_CONTROL1,
            mmio_read(EMMC_CONTROL1) | C1_CLK_INTLEN | C1_TOUNIT_MAX,
        );
        delay_us_sync(10);

        // Set clock to setup frequency
        sdhc.clk(400_000)?;
        mmio_write(EMMC_INT_EN, 0xffffffff);
        mmio_write(EMMC_INT_MASK, 0xffffffff);
        sdhc.sd_scr[0] = 0;
        sdhc.sd_scr[1] = 0;
        sdhc.sd_rca = 0;
        sdhc.cmd(CMD_GO_IDLE, 0)?;

        sdhc.cmd(CMD_SEND_IF_COND, 0x000001AA)?;
        delay(400);

        let mut got_complete = false;
        let mut got_voltage = false;
        let mut got_ccs = false;
        for _ in 0..6 {
            let response = sdhc.cmd(CMD_SEND_OP_COND, ACMD41_ARG_HC)?;
            print!("[DBUG] EMMC: CMD_SEND_OP_COND returned: 0x{:x} (", response);
            if response & ACMD41_CMD_COMPLETE != 0 {
                got_complete = true;
                print!("COMPLETE ");
            }
            if response & ACMD41_VOLTAGE != 0 {
                got_voltage = true;
                print!("VOLTAGE ");
            }
            if response & ACMD41_CMD_CCS != 0 {
                got_ccs = true;
                print!("CCS ");
            }
            println!(")");

            if got_complete {
                break;
            }

            delay(400);
        }

        if !got_complete || !got_voltage {
            return Err(());
        }

        sdhc.cmd(CMD_ALL_SEND_CID, 0)?;
        sdhc.sd_rca = sdhc.cmd(CMD_SEND_REL_ADDR, 0)?;
        println!(
            "[DBUG] EMMC: CMD_SEND_REL_ADDR returned 0x{:x}",
            sdhc.sd_rca
        );

        sdhc.clk(25_000_000)?;

        sdhc.cmd(CMD_CARD_SELECT, sdhc.sd_rca)?;

        sdhc.status(SR_DAT_INHIBIT)?;
        mmio_write(EMMC_BLKSIZECNT, (1 << 16) | 8);
        sdhc.cmd(CMD_SEND_SCR, 0)?;
        sdhc.int(INT_READ_RDY)?;

        let mut scr_part = 0;
        for _ in 0..100_000 {
            if mmio_read(EMMC_STATUS) & SR_READ_AVAILABLE != 0 {
                sdhc.sd_scr[scr_part] = mmio_read(EMMC_DATA);
                scr_part += 1;
                if scr_part == 2 {
                    break;
                }
            } else {
                delay_us_sync(1);
            }
        }
        if scr_part != 2 {
            return Err(());
        }
        if (sdhc.sd_scr[0] & SCR_SD_BUS_WIDTH_4) != 0 {
            sdhc.cmd(CMD_SET_BUS_WIDTH, sdhc.sd_rca | 2)?;
            mmio_write(EMMC_CONTROL0, mmio_read(EMMC_CONTROL0) | C0_HCTL_DWITDH);
        }

        // Add software flag
        print!("[DBUG] EMMC: supports: ");
        if (sdhc.sd_scr[0] & SCR_SUPP_SET_BLKCNT) != 0 {
            print!("SET_BLKCNT ");
        }
        if got_ccs {
            print!("CCS ");
        }
        println!();

        sdhc.sd_scr[0] &= !SCR_SUPP_CCS;
        if got_ccs {
            sdhc.sd_scr[0] |= SCR_SUPP_CCS;
        }

        println!("[INFO] EMMC: Setup success!");
        Ok(sdhc)
    }

    /// Wait for data or command ready
    pub unsafe fn status(&mut self, mask: u32) -> Result<(), ()> {
        for _ in 0..500_000 {
            if (mmio_read(EMMC_STATUS) & mask) == 0 {
                return if mmio_read(EMMC_INTERRUPT) & INT_ERROR_MASK == 0 {
                    Ok(())
                } else {
                    Err(()) // Error
                };
            } else {
                delay_us_sync(1);
            }
        }
        Err(()) // Error
    }

    /// Wait for interrupt
    pub unsafe fn int(&mut self, mask: u32) -> Result<(), ()> {
        let mut done = false;

        for _ in 0..1_000_000 {
            if mmio_read(EMMC_INTERRUPT) & (mask | INT_ERROR_MASK) == 0 {
                delay_us_sync(1);
            } else {
                done = true;
                break;
            }
        }

        let flags = mmio_read(EMMC_INTERRUPT);
        let result = if !done || (flags & INT_CMD_TIMEOUT) != 0 || (flags & INT_DATA_TIMEOUT) != 0 {
            println!("[WARN] EMMC: int timeout");
            Err(()) // Timeout
        } else if (flags & INT_ERROR_MASK) != 0 {
            println!("[WARN] EMMC: int error");
            Err(()) // Error
        } else {
            Ok(())
        };

        mmio_write(EMMC_INTERRUPT, mask);
        result
    }

    /// Send a command
    pub unsafe fn cmd(&mut self, mut code: u32, arg: u32) -> Result<u32, SdhcCmdError> {
        if (code & CMD_NEED_APP) != 0 {
            let new_code = CMD_APP_CMD | (if self.sd_rca != 0 { CMD_RSPNS_48 } else { 0 });
            let result = self.cmd(new_code, self.sd_rca)?;
            if self.sd_rca != 0 && result == 0 {
                println!("[WARN] ERROR: failed to send SD APP command");
                return Err(SdhcCmdError(0));
            }
            code &= !CMD_NEED_APP;
        }

        if self.status(SR_CMD_INHIBIT).is_err() {
            println!("[WARN] ERROR: EMMC busy");
            return Err(SdhcCmdError(0));
        }

        println!(
            "[DBUG] EMMC: Sending (command: 0x{:x}, arg: 0x{:x})",
            code, arg
        );
        mmio_write(EMMC_INTERRUPT, mmio_read(EMMC_INTERRUPT));
        mmio_write(EMMC_ARG1, arg);
        mmio_write(EMMC_CMDTM, code);

        if code == (CMD_SEND_OP_COND & !CMD_NEED_APP) {
            delay_us_sync(1000);
        } else if code == CMD_SEND_IF_COND || code == CMD_APP_CMD {
            delay_us_sync(100);
        }

        self.int(INT_CMD_DONE).map_err(|_| {
            println!("[WARN] ERROR: failed to send EMMC command");
            SdhcCmdError(0)
        })?;

        let result = mmio_read(EMMC_RESP0);

        const CMD_APP_CMD_RSPNS_48: u32 = CMD_APP_CMD | CMD_RSPNS_48;
        const CMD_SEND_OP_COND_NO_APP: u32 = CMD_SEND_OP_COND & !CMD_NEED_APP;
        match code {
            CMD_GO_IDLE | CMD_APP_CMD => Ok(0),
            CMD_APP_CMD_RSPNS_48 => Ok(result & SR_APP_CMD),
            CMD_SEND_OP_COND_NO_APP => Ok(result),
            CMD_SEND_IF_COND => {
                if result == arg {
                    Ok(result & CMD_ERRORS_MASK)
                } else {
                    Err(SdhcCmdError(result & CMD_ERRORS_MASK))
                }
            }
            CMD_ALL_SEND_CID => {
                Ok(result | mmio_read(EMMC_RESP3) | mmio_read(EMMC_RESP2) | mmio_read(EMMC_RESP1))
            }
            CMD_SEND_REL_ADDR => {
                let err = ((result & 0x1fff)
                    | ((result & 0x2000) << 6)
                    | ((result & 0x4000) << 8)
                    | ((result & 0x8000) << 8))
                    & CMD_ERRORS_MASK;
                if err != 0 {
                    Err(SdhcCmdError(result & CMD_RCA_MASK))
                } else {
                    Ok(result & CMD_RCA_MASK)
                }
            }
            _ => Ok(result & CMD_ERRORS_MASK),
        }
    }

    /// read blocks from the sd card
    pub unsafe fn read_block(&mut self, lba: u32, mut buf: &mut [u32]) -> Result<(), ()> {
        if buf.len() % (512 / 4) != 0 || buf.is_empty() {
            return Err(());
        }
        let block_count = (buf.len() / (512 / 4)) as u32;
        println!(
            "[DBUG] EMMC: read_block(lba: {}, blocks: {})",
            lba, block_count
        );

        let ccs_support = (self.sd_scr[0] & SCR_SUPP_CCS) != 0;
        let set_blkcnt_support = (self.sd_scr[0] & SCR_SUPP_SET_BLKCNT) != 0;

        self.status(SR_DAT_INHIBIT)?;
        if ccs_support {
            if block_count > 1 && set_blkcnt_support {
                self.cmd(CMD_SET_BLOCKCNT, block_count)?;
            }
            mmio_write(EMMC_BLKSIZECNT, (block_count << 16) | 512);
            self.cmd(
                if block_count == 1 {
                    CMD_READ_SINGLE
                } else {
                    CMD_READ_MULTI
                },
                lba,
            )?;
        } else {
            mmio_write(EMMC_BLKSIZECNT, (1 << 16) | 512);
        }

        for block in 0..block_count {
            if !ccs_support {
                self.cmd(CMD_READ_SINGLE, (lba + block) * 512)?;
            }
            self.int(INT_READ_RDY)?;
            for chunk in &mut buf[..128] {
                *chunk = mmio_read(EMMC_DATA);
            }
            buf = &mut buf[(512 / 4)..];
        }

        if block_count > 1 && set_blkcnt_support && ccs_support {
            self.cmd(CMD_STOP_TRANS, 0)?;
        }

        Ok(())
    }

    /// write blocks from the sd card
    #[allow(dead_code)]
    pub unsafe fn write_block(&mut self, lba: u32, mut buf: &[u32]) -> Result<(), ()> {
        if buf.len() % (512 / 4) != 0 || buf.is_empty() {
            return Err(());
        }
        let block_count = (buf.len() / (512 / 4)) as u32;
        println!(
            "[DBUG] EMMC: write_block(lba: {}, blocks: {})",
            lba, block_count
        );

        let ccs_support = (self.sd_scr[0] & SCR_SUPP_CCS) != 0;
        let set_blkcnt_support = (self.sd_scr[0] & SCR_SUPP_SET_BLKCNT) != 0;

        self.status(SR_DAT_INHIBIT | SR_WRITE_AVAILABLE)?;
        if ccs_support {
            if block_count > 1 && set_blkcnt_support {
                self.cmd(CMD_SET_BLOCKCNT, block_count)?;
            }
            mmio_write(EMMC_BLKSIZECNT, (block_count << 16) | 512);
            self.cmd(
                if block_count == 1 {
                    CMD_WRITE_SINGLE
                } else {
                    CMD_WRITE_MULTI
                },
                lba,
            )?;
        } else {
            mmio_write(EMMC_BLKSIZECNT, (1 << 16) | 512);
        }

        for block in 0..block_count {
            if !ccs_support {
                self.cmd(CMD_WRITE_SINGLE, (lba + block) * 512)?;
            }
            self.int(INT_WRITE_RDY)?;
            for chunk in &buf[..128] {
                mmio_write(EMMC_DATA, *chunk);
            }
            buf = &buf[(512 / 4)..];
        }
        self.int(INT_DATA_DONE)?;

        if block_count > 1 && set_blkcnt_support && ccs_support {
            self.cmd(CMD_STOP_TRANS, 0)?;
        }

        Ok(())
    }

    /// set SD clock to frequency in Hz
    pub unsafe fn clk(&mut self, freq: u32) -> Result<(), ()> {
        if freq == 0 {
            return Err(());
        }

        let mut done = false;
        for _ in 0..100_000 {
            if mmio_read(EMMC_STATUS) & (SR_CMD_INHIBIT | SR_DAT_INHIBIT) != 0 {
                delay_us_sync(1);
            } else {
                done = true;
                break;
            }
        }
        if !done {
            println!("[WARN] ERROR: timeout waiting for inhibit flag");
            return Err(());
        }

        mmio_write(EMMC_CONTROL1, mmio_read(EMMC_CONTROL1) & !C1_CLK_EN);
        delay_us_sync(10);

        // WTF
        let c = 41666666 / freq;
        let mut x = c - 1;
        let mut shift = 32;
        if x == 0 {
            shift = 0
        } else {
            if (x & 0xffff0000) == 0 {
                x <<= 16;
                shift -= 16;
            }
            if (x & 0xff000000) == 0 {
                x <<= 8;
                shift -= 8;
            }
            if (x & 0xf0000000) == 0 {
                x <<= 4;
                shift -= 4;
            }
            if (x & 0xc0000000) == 0 {
                x <<= 2;
                shift -= 2;
            }
            if (x & 0x80000000) == 0 {
                shift -= 1;
            }
            if shift > 0 {
                shift -= 1;
            }
            if shift > 7 {
                shift = 7;
            }
        }

        let mut divisor = if self.sd_hv > HOST_SPEC_V2 {
            c
        } else {
            1 << shift
        };
        if divisor <= 2 {
            divisor = 2;
            shift = 0;
        }
        println!("[DBUG] EMMC: sd_clk divisor {}, shift {}", divisor, shift);

        let h = if self.sd_hv > HOST_SPEC_V2 {
            (divisor & 0x300) >> 2
        } else {
            0
        };
        divisor = ((divisor & 0xff) << 8) | h;
        mmio_write(
            EMMC_CONTROL1,
            (mmio_read(EMMC_CONTROL1) & 0xffff003f) | divisor,
        );
        delay_us_sync(10);
        mmio_write(EMMC_CONTROL1, mmio_read(EMMC_CONTROL1) | C1_CLK_EN);
        delay_us_sync(10);

        let mut done = false;
        for _ in 0..10_000 {
            if mmio_read(EMMC_CONTROL1) & C1_CLK_STABLE == 0 {
                delay_us_sync(10);
            } else {
                done = true;
                break;
            }
        }
        if !done {
            println!("[WARN] ERROR: failed to get stable clock");
            return Err(());
        }

        Ok(())
    }
}
