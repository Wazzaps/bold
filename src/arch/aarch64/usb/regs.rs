use crate::arch::aarch64::mmio::{mmio_read, mmio_write, MMIO_BASE};
use c2rust_bitfields::BitfieldStruct;

pub const RECEIVE_FIFO_SIZE: u32 = 20480; /* 16 to 32768 */
pub const NON_PERIODIC_FIFO_SIZE: u32 = 20480; /* 16 to 32768 */
pub const PERIODIC_FIFO_SIZE: u32 = 20480; /* 16 to 32768 */
pub const CHANNEL_COUNT: u32 = 16;
pub const CHANNEL_SIZE: u32 = 32;
pub const REQUEST_TIMEOUT: u32 = 5000;

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct OtgControl {
    #[bitfield(name = "sesreqscs", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "sesreq", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "vbvalidoven", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "vbvalidovval", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "avalidoven", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "avalidovval", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "bvalidoven", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "bvalidovval", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "hstnegscs", ty = "u32", bits = "8..=8")]
    #[bitfield(name = "hnpreq", ty = "u32", bits = "9..=9")]
    #[bitfield(name = "HostSetHnpEnable", ty = "u32", bits = "10..=10")]
    #[bitfield(name = "devhnpen", ty = "u32", bits = "11..=11")]
    #[bitfield(name = "_reserved12_15", ty = "u32", bits = "12..=15")]
    #[bitfield(name = "conidsts", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "dbnctime", ty = "u32", bits = "17..=17")]
    #[bitfield(name = "ASessionValid", ty = "u32", bits = "18..=18")]
    #[bitfield(name = "BSessionValid", ty = "u32", bits = "19..=19")]
    #[bitfield(name = "OtgVersion", ty = "u32", bits = "20..=20")]
    #[bitfield(name = "_reserved21", ty = "u32", bits = "21..=21")]
    #[bitfield(name = "multvalidbc", ty = "u32", bits = "22..=26")]
    #[bitfield(name = "chirpen", ty = "u32", bits = "27..=27")]
    #[bitfield(name = "_reserved28_31", ty = "u32", bits = "28..=31")]
    pub data: [u8; 4],
}
impl OtgControl {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_OTG_CONTROL).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_OTG_CONTROL, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct OtgInterrupt {
    #[bitfield(name = "_reserved0_1", ty = "u32", bits = "0..=1")]
    #[bitfield(name = "SessionEndDetected", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "_reserved3_7", ty = "u32", bits = "3..=7")]
    #[bitfield(name = "SessionRequestSuccessStatusChange", ty = "u32", bits = "8..=8")]
    #[bitfield(
        name = "HostNegotiationSuccessStatusChange",
        ty = "u32",
        bits = "9..=9"
    )]
    #[bitfield(name = "_reserved10_16", ty = "u32", bits = "10..=16")]
    #[bitfield(name = "HostNegotiationDetected", ty = "u32", bits = "17..=17")]
    #[bitfield(name = "ADeviceTimeoutChange", ty = "u32", bits = "18..=18")]
    #[bitfield(name = "DebounceDone", ty = "u32", bits = "19..=19")]
    #[bitfield(name = "_reserved20_31", ty = "u32", bits = "20..=31")]
    pub data: [u8; 4],
}
impl OtgInterrupt {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_OTG_INTERRUPT).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_OTG_INTERRUPT, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct Ahb {
    #[bitfield(name = "InterruptEnable", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "DmaBurstType", ty = "DmaBurstTypeT", bits = "1..=4")]
    #[bitfield(name = "DmaEnable", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "_reserved6", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "TransferEmptyLevel", ty = "EmptyLevel", bits = "7..=7")]
    #[bitfield(name = "PeriodicTransferEmptyLevel", ty = "EmptyLevel", bits = "8..=8")]
    #[bitfield(name = "_reserved9_20", ty = "u32", bits = "9..=20")]
    #[bitfield(name = "remmemsupp", ty = "u32", bits = "21..=21")]
    #[bitfield(name = "notialldmawrit", ty = "u32", bits = "22..=22")]
    #[bitfield(name = "DmaRemainderMode", ty = "DmaRemainderModeT", bits = "23..=23")]
    #[bitfield(name = "_reserved24_31", ty = "u32", bits = "24..=31")]
    pub data: [u8; 4],
}
impl Ahb {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_AHB).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_AHB, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}

pub type DmaRemainderModeT = u32;
pub type EmptyLevel = u32;

pub const HALF: EmptyLevel = 0;
pub const EMPTY: EmptyLevel = 1;

pub type DmaBurstTypeT = u32;

pub const INCREMENTAL16: DmaBurstTypeT = 7;
pub const INCREMENTAL8: DmaBurstTypeT = 5;
pub const INCREMENTAL4: DmaBurstTypeT = 3;
pub const INCREMENTAL: DmaBurstTypeT = 1;
pub const SINGLE: DmaBurstTypeT = 0;

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct Usb {
    #[bitfield(name = "toutcal", ty = "u32", bits = "0..=2")]
    #[bitfield(name = "PhyInterface", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "ModeSelect", ty = "UMode", bits = "4..=4")]
    #[bitfield(name = "fsintf", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "physel", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "ddrsel", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "SrpCapable", ty = "u32", bits = "8..=8")]
    #[bitfield(name = "HnpCapable", ty = "u32", bits = "9..=9")]
    #[bitfield(name = "usbtrdtim", ty = "u32", bits = "10..=13")]
    #[bitfield(name = "reserved1", ty = "u32", bits = "14..=14")]
    #[bitfield(name = "phy_lpm_clk_sel", ty = "u32", bits = "15..=15")]
    #[bitfield(name = "otgutmifssel", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "UlpiFsls", ty = "u32", bits = "17..=17")]
    #[bitfield(name = "ulpi_auto_res", ty = "u32", bits = "18..=18")]
    #[bitfield(name = "ulpi_clk_sus_m", ty = "u32", bits = "19..=19")]
    #[bitfield(name = "UlpiDriveExternalVbus", ty = "u32", bits = "20..=20")]
    #[bitfield(name = "ulpi_int_vbus_indicator", ty = "u32", bits = "21..=21")]
    #[bitfield(name = "TsDlinePulseEnable", ty = "u32", bits = "22..=22")]
    #[bitfield(name = "indicator_complement", ty = "u32", bits = "23..=23")]
    #[bitfield(name = "indicator_pass_through", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "ulpi_int_prot_dis", ty = "u32", bits = "25..=25")]
    #[bitfield(name = "ic_usb_capable", ty = "u32", bits = "26..=26")]
    #[bitfield(name = "ic_traffic_pull_remove", ty = "u32", bits = "27..=27")]
    #[bitfield(name = "tx_end_delay", ty = "u32", bits = "28..=28")]
    #[bitfield(name = "force_host_mode", ty = "u32", bits = "29..=29")]
    #[bitfield(name = "force_dev_mode", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "_reserved31", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
impl Usb {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_USB).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_USB, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}
pub type UMode = u32;
pub const MODE_SELECT_UTMI: UMode = 1;
pub const MODE_SELECT_ULPI: UMode = 0;

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct CoreReset {
    #[bitfield(name = "CoreSoft", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "HclkSoft", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "HostFrameCounter", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "InTokenQueueFlush", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "ReceiveFifoFlush", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "TransmitFifoFlush", ty = "u32", bits = "5..=5")]
    #[bitfield(
        name = "TransmitFifoFlushNumber",
        ty = "CoreFifoFlush",
        bits = "6..=10"
    )]
    #[bitfield(name = "_reserved11_29", ty = "u32", bits = "11..=29")]
    #[bitfield(name = "DmaRequestSignal", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "AhbMasterIdle", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
impl CoreReset {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_RESET).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_RESET, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn clear() {
        mmio_write(HCD_DWC_RESET, 0);
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}
pub type CoreFifoFlush = u32;
pub const FLUSH_ALL: CoreFifoFlush = 16;
pub const FLUSH_PERIODIC_15: CoreFifoFlush = 15;
pub const FLUSH_PERIODIC_14: CoreFifoFlush = 14;
pub const FLUSH_PERIODIC_13: CoreFifoFlush = 13;
pub const FLUSH_PERIODIC_12: CoreFifoFlush = 12;
pub const FLUSH_PERIODIC_11: CoreFifoFlush = 11;
pub const FLUSH_PERIODIC_10: CoreFifoFlush = 10;
pub const FLUSH_PERIODIC_9: CoreFifoFlush = 9;
pub const FLUSH_PERIODIC_8: CoreFifoFlush = 8;
pub const FLUSH_PERIODIC_7: CoreFifoFlush = 7;
pub const FLUSH_PERIODIC_6: CoreFifoFlush = 6;
pub const FLUSH_PERIODIC_5: CoreFifoFlush = 5;
pub const FLUSH_PERIODIC_4: CoreFifoFlush = 4;
pub const FLUSH_PERIODIC_3: CoreFifoFlush = 3;
pub const FLUSH_PERIODIC_2: CoreFifoFlush = 2;
pub const FLUSH_PERIODIC_1: CoreFifoFlush = 1;
pub const FLUSH_NON_PERIODIC: CoreFifoFlush = 0;

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct CoreInterrupts {
    #[bitfield(name = "CurrentMode", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "ModeMismatch", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "Otg", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "DmaStartOfFrame", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "ReceiveStatusLevel", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "NpTransmitFifoEmpty", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "ginnakeff", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "goutnakeff", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "ulpick", ty = "u32", bits = "8..=8")]
    #[bitfield(name = "I2c", ty = "u32", bits = "9..=9")]
    #[bitfield(name = "EarlySuspend", ty = "u32", bits = "10..=10")]
    #[bitfield(name = "UsbSuspend", ty = "u32", bits = "11..=11")]
    #[bitfield(name = "UsbReset", ty = "u32", bits = "12..=12")]
    #[bitfield(name = "EnumerationDone", ty = "u32", bits = "13..=13")]
    #[bitfield(name = "IsochronousOutDrop", ty = "u32", bits = "14..=14")]
    #[bitfield(name = "eopframe", ty = "u32", bits = "15..=15")]
    #[bitfield(name = "RestoreDone", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "EndPointMismatch", ty = "u32", bits = "17..=17")]
    #[bitfield(name = "InEndPoint", ty = "u32", bits = "18..=18")]
    #[bitfield(name = "OutEndPoint", ty = "u32", bits = "19..=19")]
    #[bitfield(name = "IncompleteIsochronousIn", ty = "u32", bits = "20..=20")]
    #[bitfield(name = "IncompleteIsochronousOut", ty = "u32", bits = "21..=21")]
    #[bitfield(name = "fetsetup", ty = "u32", bits = "22..=22")]
    #[bitfield(name = "ResetDetect", ty = "u32", bits = "23..=23")]
    #[bitfield(name = "Port", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "HostChannel", ty = "u32", bits = "25..=25")]
    #[bitfield(name = "HpTransmitFifoEmpty", ty = "u32", bits = "26..=26")]
    #[bitfield(name = "LowPowerModeTransmitReceived", ty = "u32", bits = "27..=27")]
    #[bitfield(name = "ConnectionIdStatusChange", ty = "u32", bits = "28..=28")]
    #[bitfield(name = "Disconnect", ty = "u32", bits = "29..=29")]
    #[bitfield(name = "SessionRequest", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "Wakeup", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
impl CoreInterrupts {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_INTERRUPT).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_INTERRUPT, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
    pub unsafe fn get_mask() -> Self {
        Self {
            data: mmio_read(HCD_DWC_INTERRUPT_MASK).to_ne_bytes(),
        }
    }
    pub unsafe fn set_mask(self) {
        mmio_write(HCD_DWC_INTERRUPT_MASK, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify_mask<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get_mask();
        (f)(&mut val);
        val.set_mask();
    }
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct ChannelInterrupts {
    #[bitfield(name = "TransferComplete", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "Halt", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "AhbError", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "Stall", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "NegativeAcknowledgement", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "Acknowledgement", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "NotYet", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "TransactionError", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "BabbleError", ty = "u32", bits = "8..=8")]
    #[bitfield(name = "FrameOverrun", ty = "u32", bits = "9..=9")]
    #[bitfield(name = "DataToggleError", ty = "u32", bits = "10..=10")]
    #[bitfield(name = "BufferNotAvailable", ty = "u32", bits = "11..=11")]
    #[bitfield(name = "ExcessiveTransmission", ty = "u32", bits = "12..=12")]
    #[bitfield(name = "FrameListRollover", ty = "u32", bits = "13..=13")]
    #[bitfield(name = "_reserved14_31", ty = "u32", bits = "14..=31")]
    pub data: [u8; 4],
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct Receive {
    pub peek: ReceiveStatus,
    pub pop: ReceiveStatus,
    pub size: u32,
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct ReceiveStatus {
    #[bitfield(name = "ChannelNumber", ty = "u32", bits = "0..=3")]
    #[bitfield(name = "bcnt", ty = "u32", bits = "4..=14")]
    #[bitfield(name = "dpid", ty = "u32", bits = "15..=16")]
    #[bitfield(name = "PacketStatus", ty = "PacketStatus", bits = "17..=20")]
    #[bitfield(name = "_reserved21_31", ty = "u32", bits = "21..=31")]
    pub data: [u8; 4],
}
pub type PacketStatus = u32;
pub const CHANNEL_HALTED: PacketStatus = 7;
pub const DATA_TOGGLE_ERROR: PacketStatus = 5;
pub const IN_TRANSFER_COMPLETE: PacketStatus = 3;
pub const IN_PACKET: PacketStatus = 2;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct NonPeriodicFifo {
    pub size: FifoSize,
    pub status: Status,
}
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct Status {
    #[bitfield(name = "SpaceAvailable", ty = "u32", bits = "0..=15")]
    #[bitfield(name = "QueueSpaceAvailable", ty = "u32", bits = "16..=23")]
    #[bitfield(name = "Terminate", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "TokenType", ty = "StatusTokenType", bits = "25..=26")]
    #[bitfield(name = "Channel", ty = "u32", bits = "27..=30")]
    #[bitfield(name = "Odd", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
pub type StatusTokenType = u32;
pub const CHANNEL_HALT: StatusTokenType = 3;
pub const PING_COMPLETE_SPLIT: StatusTokenType = 2;
pub const ZERO_LENGTH_OUT: StatusTokenType = 1;
pub const IN_OUT: StatusTokenType = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct FifoSize {
    #[bitfield(name = "StartAddress", ty = "u32", bits = "0..=15")]
    #[bitfield(name = "Depth", ty = "u32", bits = "16..=31")]
    pub data: [u8; 4],
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct I2cControl {
    #[bitfield(name = "ReadWriteData", ty = "u32", bits = "0..=7")]
    #[bitfield(name = "RegisterAddress", ty = "u32", bits = "8..=15")]
    #[bitfield(name = "Address", ty = "u32", bits = "16..=22")]
    #[bitfield(name = "I2cEnable", ty = "u32", bits = "23..=23")]
    #[bitfield(name = "Acknowledge", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "I2cSuspendControl", ty = "u32", bits = "25..=25")]
    #[bitfield(name = "I2cDeviceAddress", ty = "u32", bits = "26..=27")]
    #[bitfield(name = "_reserved28_29", ty = "u32", bits = "28..=29")]
    #[bitfield(name = "ReadWrite", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "bsydne", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct Hardware {
    #[bitfield(name = "Direction0", ty = "u32", bits = "0..=1")]
    #[bitfield(name = "Direction1", ty = "u32", bits = "2..=3")]
    #[bitfield(name = "Direction2", ty = "u32", bits = "4..=5")]
    #[bitfield(name = "Direction3", ty = "u32", bits = "6..=7")]
    #[bitfield(name = "Direction4", ty = "u32", bits = "8..=9")]
    #[bitfield(name = "Direction5", ty = "u32", bits = "10..=11")]
    #[bitfield(name = "Direction6", ty = "u32", bits = "12..=13")]
    #[bitfield(name = "Direction7", ty = "u32", bits = "14..=15")]
    #[bitfield(name = "Direction8", ty = "u32", bits = "16..=17")]
    #[bitfield(name = "Direction9", ty = "u32", bits = "18..=19")]
    #[bitfield(name = "Direction10", ty = "u32", bits = "20..=21")]
    #[bitfield(name = "Direction11", ty = "u32", bits = "22..=23")]
    #[bitfield(name = "Direction12", ty = "u32", bits = "24..=25")]
    #[bitfield(name = "Direction13", ty = "u32", bits = "26..=27")]
    #[bitfield(name = "Direction14", ty = "u32", bits = "28..=29")]
    #[bitfield(name = "Direction15", ty = "u32", bits = "30..=31")]
    #[bitfield(name = "OperatingMode", ty = "OperatingMode", bits = "32..=34")]
    #[bitfield(name = "Architecture", ty = "Architecture", bits = "35..=36")]
    #[bitfield(name = "PointToPoint", ty = "u32", bits = "37..=37")]
    #[bitfield(name = "HighSpeedPhysical", ty = "HighSpeedPhysical", bits = "38..=39")]
    #[bitfield(name = "FullSpeedPhysical", ty = "FullSpeedPhysical", bits = "40..=41")]
    #[bitfield(name = "DeviceEndPointCount", ty = "u32", bits = "42..=45")]
    #[bitfield(name = "HostChannelCount", ty = "u32", bits = "46..=49")]
    #[bitfield(name = "SupportsPeriodicEndpoints", ty = "u32", bits = "50..=50")]
    #[bitfield(name = "DynamicFifo", ty = "u32", bits = "51..=51")]
    #[bitfield(name = "multi_proc_int", ty = "u32", bits = "52..=52")]
    #[bitfield(name = "_reserver21", ty = "u32", bits = "53..=53")]
    #[bitfield(name = "NonPeriodicQueueDepth", ty = "u32", bits = "54..=55")]
    #[bitfield(name = "HostPeriodicQueueDepth", ty = "u32", bits = "56..=57")]
    #[bitfield(name = "DeviceTokenQueueDepth", ty = "u32", bits = "58..=62")]
    #[bitfield(name = "EnableIcUsb", ty = "u32", bits = "63..=63")]
    #[bitfield(name = "TransferSizeControlWidth", ty = "u32", bits = "64..=67")]
    #[bitfield(name = "PacketSizeControlWidth", ty = "u32", bits = "68..=70")]
    #[bitfield(name = "otg_func", ty = "u32", bits = "71..=71")]
    #[bitfield(name = "I2c", ty = "u32", bits = "72..=72")]
    #[bitfield(name = "VendorControlInterface", ty = "u32", bits = "73..=73")]
    #[bitfield(name = "OptionalFeatures", ty = "u32", bits = "74..=74")]
    #[bitfield(name = "SynchronousResetType", ty = "u32", bits = "75..=75")]
    #[bitfield(name = "AdpSupport", ty = "u32", bits = "76..=76")]
    #[bitfield(name = "otg_enable_hsic", ty = "u32", bits = "77..=77")]
    #[bitfield(name = "bc_support", ty = "u32", bits = "78..=78")]
    #[bitfield(name = "LowPowerModeEnabled", ty = "u32", bits = "79..=79")]
    #[bitfield(name = "FifoDepth", ty = "u32", bits = "80..=95")]
    #[bitfield(name = "PeriodicInEndpointCount", ty = "u32", bits = "96..=99")]
    #[bitfield(name = "PowerOptimisation", ty = "u32", bits = "100..=100")]
    #[bitfield(name = "MinimumAhbFrequency", ty = "u32", bits = "101..=101")]
    #[bitfield(name = "PartialPowerOff", ty = "u32", bits = "102..=102")]
    #[bitfield(name = "_reserved103_109", ty = "u32", bits = "103..=109")]
    #[bitfield(
        name = "UtmiPhysicalDataWidth",
        ty = "UtmiPhysicalDataWidth",
        bits = "110..=111"
    )]
    #[bitfield(name = "ModeControlEndpointCount", ty = "u32", bits = "112..=115")]
    #[bitfield(name = "ValidFilterIddigEnabled", ty = "u32", bits = "116..=116")]
    #[bitfield(name = "VbusValidFilterEnabled", ty = "u32", bits = "117..=117")]
    #[bitfield(name = "ValidFilterAEnabled", ty = "u32", bits = "118..=118")]
    #[bitfield(name = "ValidFilterBEnabled", ty = "u32", bits = "119..=119")]
    #[bitfield(name = "SessionEndFilterEnabled", ty = "u32", bits = "120..=120")]
    #[bitfield(name = "ded_fifo_en", ty = "u32", bits = "121..=121")]
    #[bitfield(name = "InEndpointCount", ty = "u32", bits = "122..=125")]
    #[bitfield(name = "DmaDescription", ty = "u32", bits = "126..=126")]
    #[bitfield(name = "DmaDynamicDescription", ty = "u32", bits = "127..=127")]
    pub data: [u8; 16],
}
pub type UtmiPhysicalDataWidth = u32;
pub const WIDTH_8_OR_16_BIT: UtmiPhysicalDataWidth = 2;
pub const WIDTH_16_BIT: UtmiPhysicalDataWidth = 1;
pub const WIDTH_8_BIT: UtmiPhysicalDataWidth = 0;
pub type FullSpeedPhysical = u32;
pub const PHYSICAL_3: FullSpeedPhysical = 3;
pub const PHYSICAL_2: FullSpeedPhysical = 2;
pub const DEDICATED: FullSpeedPhysical = 1;
pub const PHYSICAL_0: FullSpeedPhysical = 0;
pub type HighSpeedPhysical = u32;
pub const HSP_UTMI_ULPI: HighSpeedPhysical = 3;
pub const HSP_ULPI: HighSpeedPhysical = 2;
pub const HSP_UTMI: HighSpeedPhysical = 1;
pub const HSP_NOT_SUPPORTED: HighSpeedPhysical = 0;
pub type Architecture = u32;
pub const INTERNAL_DMA: Architecture = 2;
pub const EXTERNAL_DMA: Architecture = 1;
pub const SLAVE_ONLY: Architecture = 0;
pub type OperatingMode = u32;
pub const NO_SRP_CAPABLE_HOST: OperatingMode = 6;
pub const SRP_CAPABLE_HOST: OperatingMode = 5;
pub const NO_SRP_CAPABLE_DEVICE: OperatingMode = 4;
pub const SRP_CAPABLE_DEVICE: OperatingMode = 3;
pub const NO_HNP_SRP_CAPABLE: OperatingMode = 2;
pub const SRP_ONLY_CAPABLE: OperatingMode = 1;
pub const HNP_SRP_CAPABLE: OperatingMode = 0;
pub type UsbSpeed = u32;
pub const USB_SPEED_LOW: UsbSpeed = 2;
pub const USB_SPEED_FULL: UsbSpeed = 1;
pub const USB_SPEED_HIGH: UsbSpeed = 0;
pub type UsbDirection = u32;
pub const EP_DIR_IN: UsbDirection = 1;
pub const EP_DIR_DEVICE_TO_HOST: UsbDirection = 1;
pub const EP_DIR_OUT: UsbDirection = 0;
pub const EP_DIR_HOST_TO_DEVICE: UsbDirection = 0;
pub type UsbTransfer = u32;
pub const USB_XFER_INTERRUPT: UsbTransfer = 3;
pub const USB_XFER_BULK: UsbTransfer = 2;
pub const USB_XFER_ISOCHRONOUS: UsbTransfer = 1;
pub const USB_XFER_CONTROL: UsbTransfer = 0;

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct LowPowerModeConfiguration {
    #[bitfield(name = "LowPowerModeCapable", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "ApplicationResponse", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "HostInitiatedResumeDuration", ty = "u32", bits = "2..=5")]
    #[bitfield(name = "RemoteWakeupEnabled", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "UtmiSleepEnabled", ty = "u32", bits = "7..=7")]
    #[bitfield(
        name = "HostInitiatedResumeDurationThreshold",
        ty = "u32",
        bits = "8..=12"
    )]
    #[bitfield(name = "LowPowerModeResponse", ty = "u32", bits = "13..=14")]
    #[bitfield(name = "PortSleepStatus", ty = "u32", bits = "15..=15")]
    #[bitfield(name = "SleepStateResumeOk", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "LowPowerModeChannelIndex", ty = "u32", bits = "17..=20")]
    #[bitfield(name = "RetryCount", ty = "u32", bits = "21..=23")]
    #[bitfield(name = "SendLowPowerMode", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "RetryCountStatus", ty = "u32", bits = "25..=27")]
    #[bitfield(name = "_reserved28_29", ty = "u32", bits = "28..=29")]
    #[bitfield(name = "HsicConnect", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "InverseSelectHsic", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct PeriodicFifo {
    pub host_size: FifoSize,
    pub data_size: [FifoSize; 15],
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct PowerReg {
    #[bitfield(name = "StopPClock", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "GateHClock", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "PowerClamp", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "PowerDownModules", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "PhySuspended", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "EnableSleepClockGating", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "PhySleeping", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "DeepSleep", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "_reserved8_31", ty = "u32", bits = "8..=31")]
    pub data: [u8; 4],
}

impl PowerReg {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_POWER).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_POWER, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn clear() {
        mmio_write(HCD_DWC_POWER, 0);
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}

#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct HostConfig {
    #[bitfield(name = "ClockRate", ty = "ClockRate", bits = "0..=1")]
    #[bitfield(name = "FslsOnly", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "_reserved3_6", ty = "u32", bits = "3..=6")]
    #[bitfield(name = "en_32khz_susp", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "res_val_period", ty = "u32", bits = "8..=15")]
    #[bitfield(name = "_reserved16_22", ty = "u32", bits = "16..=22")]
    #[bitfield(name = "EnableDmaDescriptor", ty = "u32", bits = "23..=23")]
    #[bitfield(name = "FrameListEntries", ty = "u32", bits = "24..=25")]
    #[bitfield(name = "PeriodicScheduleEnable", ty = "u32", bits = "26..=26")]
    #[bitfield(name = "PeriodicScheduleStatus", ty = "u32", bits = "27..=27")]
    #[bitfield(name = "reserved28_30", ty = "u32", bits = "28..=30")]
    #[bitfield(name = "mode_chg_time", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
impl HostConfig {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_HOST).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        mmio_write(HCD_DWC_HOST, u32::from_ne_bytes(self.data));
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}
pub type ClockRate = u32;
pub const CLOCK_6_MHZ: ClockRate = 2;
pub const CLOCK_48_MHZ: ClockRate = 1;
pub const CLOCK_30_60_MHZ: ClockRate = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct FrameInterval {
    #[bitfield(name = "Interval", ty = "u32", bits = "0..=15")]
    #[bitfield(name = "DynamicFrameReload", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "_reserved17_31", ty = "u32", bits = "17..=31")]
    pub data: [u8; 4],
}
// +0x404
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct FrameNumber {
    #[bitfield(name = "FrameNumber", ty = "u32", bits = "0..=15")]
    #[bitfield(name = "FrameRemaining", ty = "u32", bits = "16..=31")]
    pub data: [u8; 4],
}
// +0x408
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct FifoStatus {
    #[bitfield(name = "SpaceAvailable", ty = "u32", bits = "0..=15")]
    #[bitfield(name = "QueueSpaceAvailable", ty = "u32", bits = "16..=23")]
    #[bitfield(name = "Terminate", ty = "u32", bits = "24..=24")]
    #[bitfield(name = "TokenType", ty = "FifoTokenType", bits = "25..=26")]
    #[bitfield(name = "Channel", ty = "u32", bits = "27..=30")]
    #[bitfield(name = "Odd", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
pub type FifoTokenType = u32;
pub const DISABLE: FifoTokenType = 2;
pub const PING: FifoTokenType = 1;
pub const ZERO_LENGTH: FifoTokenType = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct HostPort {
    #[bitfield(name = "Connect", ty = "u32", bits = "0..=0")]
    #[bitfield(name = "ConnectDetected", ty = "u32", bits = "1..=1")]
    #[bitfield(name = "Enable", ty = "u32", bits = "2..=2")]
    #[bitfield(name = "EnableChanged", ty = "u32", bits = "3..=3")]
    #[bitfield(name = "OverCurrent", ty = "u32", bits = "4..=4")]
    #[bitfield(name = "OverCurrentChanged", ty = "u32", bits = "5..=5")]
    #[bitfield(name = "Resume", ty = "u32", bits = "6..=6")]
    #[bitfield(name = "Suspend", ty = "u32", bits = "7..=7")]
    #[bitfield(name = "Reset", ty = "u32", bits = "8..=8")]
    #[bitfield(name = "_reserved9", ty = "u32", bits = "9..=9")]
    #[bitfield(name = "PortLineStatus", ty = "u32", bits = "10..=11")]
    #[bitfield(name = "Power", ty = "u32", bits = "12..=12")]
    #[bitfield(name = "TestControl", ty = "u32", bits = "13..=16")]
    #[bitfield(name = "Speed", ty = "UsbSpeed", bits = "17..=18")]
    #[bitfield(name = "_reserved19_31", ty = "u32", bits = "19..=31")]
    pub data: [u8; 4],
}
impl HostPort {
    pub unsafe fn get() -> Self {
        Self {
            data: mmio_read(HCD_DWC_HOST).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self) {
        // Don't write to read-only bits
        mmio_write(HCD_DWC_HOST, u32::from_ne_bytes(self.data) & 0x1f140);
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(f: F) {
        let mut val = Self::get();
        (f)(&mut val);
        val.set();
    }
}
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct HostChannel {
    pub characteristic: HostChannelCharacteristic,
    pub split_control: HCSplitControl,
    pub interrupt: ChannelInterrupts,
    pub interrupt_mask: ChannelInterrupts,
    pub transfer_size: HCXferSize,
    pub dma_address: u32,
    pub _reserved18: u32,
    pub _reserved1c: u32,
}
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct HCXferSize {
    #[bitfield(name = "TransferSize", ty = "u32", bits = "0..=18")]
    #[bitfield(name = "PacketCount", ty = "u32", bits = "19..=28")]
    #[bitfield(name = "PacketId", ty = "PacketId", bits = "29..=30")]
    #[bitfield(name = "DoPing", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
pub type PacketId = u32;
pub const SETUP: PacketId = 3;
pub const MDATA: PacketId = 3;
pub const DATA_2: PacketId = 1;
pub const DATA_1: PacketId = 2;
pub const DATA_0: PacketId = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct HCSplitControl {
    #[bitfield(name = "PortAddress", ty = "u32", bits = "0..=6")]
    #[bitfield(name = "HubAddress", ty = "u32", bits = "7..=13")]
    #[bitfield(
        name = "TransactionPosition",
        ty = "HcTransactionPosition",
        bits = "14..=15"
    )]
    #[bitfield(name = "CompleteSplit", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "_reserved17_30", ty = "u32", bits = "17..=30")]
    #[bitfield(name = "SplitEnable", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
pub type HcTransactionPosition = u32;
pub const HCTP_ALL: HcTransactionPosition = 3;
pub const HCTP_BEGIN: HcTransactionPosition = 2;
pub const HCTP_END: HcTransactionPosition = 1;
pub const HCTP_MIDDLE: HcTransactionPosition = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C, packed)]
pub struct HostChannelCharacteristic {
    #[bitfield(name = "MaximumPacketSize", ty = "u32", bits = "0..=10")]
    #[bitfield(name = "EndPointNumber", ty = "u32", bits = "11..=14")]
    #[bitfield(name = "EndPointDirection", ty = "UsbDirection", bits = "15..=15")]
    #[bitfield(name = "_reserved16", ty = "u32", bits = "16..=16")]
    #[bitfield(name = "LowSpeed", ty = "u32", bits = "17..=17")]
    #[bitfield(name = "Type", ty = "UsbTransfer", bits = "18..=19")]
    #[bitfield(name = "PacketsPerFrame", ty = "u32", bits = "20..=21")]
    #[bitfield(name = "DeviceAddress", ty = "u32", bits = "22..=28")]
    #[bitfield(name = "OddFrame", ty = "u32", bits = "29..=29")]
    #[bitfield(name = "Disable", ty = "u32", bits = "30..=30")]
    #[bitfield(name = "Enable", ty = "u32", bits = "31..=31")]
    pub data: [u8; 4],
}
impl HostChannelCharacteristic {
    pub unsafe fn get(channel: u32) -> Self {
        Self {
            data: mmio_read(HCD_DWC_CHANNEL_CHARACTERISTIC + channel * CHANNEL_SIZE).to_ne_bytes(),
        }
    }
    pub unsafe fn set(self, channel: u32) {
        mmio_write(
            HCD_DWC_CHANNEL_CHARACTERISTIC + channel * CHANNEL_SIZE,
            u32::from_ne_bytes(self.data),
        );
    }
    pub unsafe fn modify<F: Fn(&mut Self)>(channel: u32, f: F) {
        let mut val = Self::get(channel);
        (f)(&mut val);
        val.set(channel);
    }
}

pub const HCD_DWC_BASE: u32 = MMIO_BASE + 0x980000;
pub const HCD_DWC_OTG_CONTROL: u32 = HCD_DWC_BASE;
pub const HCD_DWC_OTG_INTERRUPT: u32 = HCD_DWC_BASE + 0x4;
pub const HCD_DWC_AHB: u32 = HCD_DWC_BASE + 0x8;
pub const HCD_DWC_USB: u32 = HCD_DWC_BASE + 0xc;
pub const HCD_DWC_RESET: u32 = HCD_DWC_BASE + 0x10;
pub const HCD_DWC_INTERRUPT: u32 = HCD_DWC_BASE + 0x14;
pub const HCD_DWC_INTERRUPT_MASK: u32 = HCD_DWC_BASE + 0x18;
pub const HCD_DWC_RECEIVE_SIZE: u32 = HCD_DWC_BASE + 0x24;
pub const HCD_DWC_NON_PERIODIC_FIFO_SIZE: u32 = HCD_DWC_BASE + 0x28;
pub const HCD_DWC_VENDOR_ID: u32 = HCD_DWC_BASE + 0x40;
pub const HCD_DWC_HARDWARE: u32 = HCD_DWC_BASE + 0x44;
pub const HCD_DWC_PERIODIC_FIFO_SIZE: u32 = HCD_DWC_BASE + 0x100;
pub const HCD_DWC_HOST: u32 = HCD_DWC_BASE + 0x400;
pub const HCD_DWC_HOST_PORT: u32 = HCD_DWC_BASE + 0x440;
pub const HCD_DWC_CHANNEL_BASE: u32 = HCD_DWC_BASE + 0x500;
pub const HCD_DWC_CHANNEL_CHARACTERISTIC: u32 = HCD_DWC_CHANNEL_BASE;
pub const HCD_DWC_POWER: u32 = HCD_DWC_BASE + 0xe00;
