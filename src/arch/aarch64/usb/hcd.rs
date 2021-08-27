use crate::arch::aarch64::mmio::{delay_us_sync, mmio_read, mmio_write};
use crate::arch::aarch64::usb::regs;
use crate::prelude::*;

unsafe fn power_on_usb() -> Result<(), ()> {
    // TODO: Test on hardware
    // let state = mailbox_methods::get_power_state(mailbox_methods::PWR_DEV_USB_HCD).unwrap();
    // let timing = mailbox_methods::get_power_timing(mailbox_methods::PWR_DEV_USB_HCD).unwrap();
    // println!("timing = {:x}, state = {:x}", timing, state);
    // mailbox_methods::set_power_state(mailbox_methods::PWR_DEV_USB_HCD, 1).unwrap();
    // delay_us_sync(timing as u64);
    //
    // let state = mailbox_methods::get_power_state(mailbox_methods::PWR_DEV_USB_HCD).unwrap();
    // println!("state 2 = {:x}", state);
    Ok(())
}

pub unsafe fn init() -> Result<(), ()> {
    let vendor_id = mmio_read(regs::HCD_DWC_VENDOR_ID);
    // 'OT'2
    if (vendor_id & 0xfffff000) != 0x4f542000 {
        println!(
            "[WARN] USB: HCD: Hardware = {}{}{:x}.{:x}{:x}{:x}. Driver incompatible. Expected OT2.xxx.",
            char::from((vendor_id >> 24) as u8),
            char::from((vendor_id >> 16) as u8),
            (vendor_id >> 12) & 0xf,
            (vendor_id >> 8) & 0xf,
            (vendor_id >> 4) & 0xf,
            (vendor_id >> 0) & 0xf,
        );

        return Err(());
    } else {
        println!(
            "[INFO] USB: HCD: Hardware = {}{}{:x}.{:x}{:x}{:x}.",
            char::from((vendor_id >> 24) as u8),
            char::from((vendor_id >> 16) as u8),
            (vendor_id >> 12) & 0xf,
            (vendor_id >> 8) & 0xf,
            (vendor_id >> 4) & 0xf,
            (vendor_id >> 0) & 0xf,
        );
    }

    let mut hw_bytes = [0u8; 16];
    hw_bytes[0..4].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE).to_ne_bytes());
    hw_bytes[4..8].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 4).to_ne_bytes());
    hw_bytes[8..12].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 8).to_ne_bytes());
    hw_bytes[12..16].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 12).to_ne_bytes());
    println!("{:?}", hw_bytes);

    println!(
        "[INFO] USB: HCD: Hardware configuration = {:08x} {:08x} {:08x} {:08x}",
        mmio_read(regs::HCD_DWC_HARDWARE),
        mmio_read(regs::HCD_DWC_HARDWARE + 4),
        mmio_read(regs::HCD_DWC_HARDWARE + 8),
        mmio_read(regs::HCD_DWC_HARDWARE + 12),
    );
    println!(
        "[INFO] USB: HCD: Host configuration = {:08x}",
        mmio_read(regs::HCD_DWC_HOST),
    );

    let hardware = regs::Hardware { data: hw_bytes };
    if hardware.Architecture() != regs::INTERNAL_DMA {
        println!(
            "[WARN] USB: HCD: Host architecture is not Internal DMA (is {}). Driver incompatible.",
            hardware.Architecture()
        );
        return Err(());
    }
    println!("[INFO] USB: HCD: Host architecture is Internal DMA.");

    // FIXME:
    // let hardware_high_speed_physical = (mmio_read(regs::HCD_DWC_HARDWARE + 1) >> 25) & 0b11;
    // if hardware.HighSpeedPhysical() == regs::HSP_NOT_SUPPORTED {
    //     println!("[WARN] USB: HCD: High speed physical unsupported. Driver incompatible.");
    //     return Err(());
    // }

    println!("[INFO] USB: HCD: Disabling interrupts.");
    mmio_write(regs::HCD_DWC_INTERRUPT_MASK, 0);
    regs::Ahb::modify(|ahb| ahb.set_InterruptEnable(0));

    println!("[INFO] USB: HCD: Powering USB on.");
    if let Err(_) = power_on_usb() {
        println!("[WARN] USB: HCD: Failed to power on USB Host Controller.");
        return Err(());
    }

    println!("[INFO] USB: HCD: Load completed.");

    Ok(())
}

unsafe fn hcd_reset() -> Result<(), ()> {
    // do {
    //     ReadBackReg(&Core->Reset);
    //     if (count++ >= 0x100000) {
    //       LOG("HCD: Device Hang!\n");
    //       return ErrorDevice;
    //     }
    //   } while (Core->Reset.AhbMasterIdle == false);
    let mut got_idle: bool = false;
    for _ in 0..0x100000 {
        // println!(
        //     "mmio_read(HCD_DWC_RESET) = 0x{:x}",
        //     mmio_read(HCD_DWC_RESET)
        // );
        if regs::CoreReset::get().AhbMasterIdle() != 0 {
            got_idle = true;
            break;
        }
    }
    if !got_idle {
        println!("[WARN] USB: HCD: Device hang! 1");
        return Err(());
    }

    //   Core->Reset.CoreSoft = true;
    //   WriteThroughReg(&Core->Reset);
    regs::CoreReset::modify(|r| r.set_CoreSoft(1));

    //   count = 0;
    //   do {
    //     ReadBackReg(&Core->Reset);
    //     if (count++ >= 0x100000) {
    //       LOG("HCD: Device Hang!\n");
    //       return ErrorDevice;
    //     }
    //   } while (Core->Reset.CoreSoft == true || Core->Reset.AhbMasterIdle == false);
    let mut got_idle: bool = false;
    for _ in 0..0x100000 {
        let reset_state = regs::CoreReset::get();
        if (reset_state.CoreSoft() == 0) && (reset_state.AhbMasterIdle() != 0) {
            got_idle = true;
            break;
        }
    }
    if !got_idle {
        println!("[WARN] USB: HCD: Device hang! 2");
        return Err(());
    }

    Ok(())
}

// Result HcdTransmitFifoFlush(enum CoreFifoFlush fifo) {
//   u32 count = 0;
unsafe fn hcd_transmit_fifo_flush(fifo: regs::CoreFifoFlush) -> Result<(), ()> {
    //   if (fifo == FlushAll)
    //     LOG_DEBUG("HCD: TXFlush(All)\n");
    //   else if (fifo == FlushNonPeriodic)
    //     LOG_DEBUG("HCD: TXFlush(NP)\n");
    //   else
    //     LOG_DEBUGF("HCD: TXFlush(P%u)\n", fifo);
    //
    if fifo == regs::FLUSH_ALL {
        println!("HCD: TXFlush(All)");
    } else if fifo == regs::FLUSH_NON_PERIODIC {
        println!("HCD: TXFlush(NP)");
    } else {
        println!("HCD: TXFlush(P{})", fifo);
    }

    //   ClearReg(&Core->Reset);
    let mut core_reset = regs::CoreReset { data: [0; 4] };
    //   Core->Reset.TransmitFifoFlushNumber = fifo;
    core_reset.set_TransmitFifoFlushNumber(fifo);
    //   Core->Reset.TransmitFifoFlush = true;
    core_reset.set_TransmitFifoFlush(1);
    //   WriteThroughReg(&Core->Reset);
    core_reset.set();

    //   count = 0;
    //
    //   do {
    //     ReadBackReg(&Core->Reset);
    //     if (count++ >= 0x100000) {
    //       LOG("HCD: Device Hang!\n");
    //       return ErrorDevice;
    //     }
    //   } while (Core->Reset.TransmitFifoFlush == true);
    let mut got_flush: bool = false;
    for _ in 0..0x100000 {
        if regs::CoreReset::get().TransmitFifoFlush() == 0 {
            got_flush = true;
            break;
        }
    }
    if !got_flush {
        println!("[WARN] USB: HCD: Device hang! 3");
        return Err(());
    }

    //   return OK;
    Ok(())
}

// Result HcdReceiveFifoFlush() {
//   u32 count = 0;
unsafe fn hcd_receive_fifo_flush() -> Result<(), ()> {
    //   LOG_DEBUG("HCD: RXFlush(All)\n");
    println!("HCD: RXFlush(All)");

    //   ClearReg(&Core->Reset);
    let mut core_reset = regs::CoreReset { data: [0; 4] };
    //   Core->Reset.ReceiveFifoFlush = true;
    core_reset.set_ReceiveFifoFlush(1);
    //   WriteThroughReg(&Core->Reset);
    core_reset.set();

    //   count = 0;
    //
    //   do {
    //     ReadBackReg(&Core->Reset);
    //     if (count++ >= 0x100000) {
    //       LOG("HCD: Device Hang!\n");
    //       return ErrorDevice;
    //     }
    //   } while (Core->Reset.ReceiveFifoFlush == true);
    let mut got_flush: bool = false;
    for _ in 0..0x100000 {
        if regs::CoreReset::get().ReceiveFifoFlush() == 0 {
            got_flush = true;
            break;
        }
    }
    if !got_flush {
        println!("[WARN] USB: HCD: Device hang! 4");
        return Err(());
    }

    //   return OK;
    Ok(())
}

pub unsafe fn start() -> Result<(), ()> {
    //  LOG_DEBUG("HCD: Start core.\n");
    println!("[INFO] USB: HCD: Start core.");
    //   if (Core == NULL) {
    //     LOG("HCD: HCD uninitialised. Cannot be started.\n");
    //     return ErrorDevice;
    //   }
    //
    // TODO:
    //   if ((databuffer = MemoryAllocate(1024)) == NULL)
    //     return ErrorMemory;
    //
    //   ReadBackReg(&Core->Usb);
    //   Core->Usb.UlpiDriveExternalVbus = 0;
    //   Core->Usb.TsDlinePulseEnable = 0;
    //   WriteThroughReg(&Core->Usb);
    mmio_write(regs::HCD_DWC_USB, {
        let mut usb = regs::Usb {
            data: mmio_read(regs::HCD_DWC_USB).to_ne_bytes(),
        };
        usb.set_UlpiDriveExternalVbus(0);
        usb.set_TsDlinePulseEnable(0);
        u32::from_ne_bytes(usb.data)
    });

    // LOG_DEBUG("HCD: Master reset.\n");
    println!("[INFO] USB: HCD: Master reset.");

    // if ((result = HcdReset()) != OK) {
    //   goto deallocate;
    // }
    hcd_reset()?;

    // if (!g_phy_initialised) {
    //   LOG_DEBUG("HCD: One time phy initialisation.\n");
    //   g_phy_initialised = true;
    println!("HCD: One time phy initialisation.");

    //     LOG_DEBUG("HCD: Interface: UTMI+.\n");
    println!("HCD: Interface: UTMI+.");

    //     Core->Usb.ModeSelect = UTMI;
    //     Core->Usb.PhyInterface = false;
    regs::Usb::modify(|usb| {
        usb.set_ModeSelect(regs::MODE_SELECT_UTMI);
        usb.set_PhyInterface(0);
    });

    //     WriteThroughReg(&Core->Usb);
    //     HcdReset();
    hcd_reset()?;
    // }

    //   ReadBackReg(&Core->Usb);
    let hardware = {
        let mut hw_bytes = [0u8; 16];
        hw_bytes[0..4].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE).to_ne_bytes());
        hw_bytes[4..8].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 4).to_ne_bytes());
        hw_bytes[8..12].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 8).to_ne_bytes());
        hw_bytes[12..16].copy_from_slice(&mmio_read(regs::HCD_DWC_HARDWARE + 12).to_ne_bytes());
        regs::Hardware { data: hw_bytes }
    };
    //   if (Core->Hardware.HighSpeedPhysical == Ulpi
    //     && Core->Hardware.FullSpeedPhysical == Dedicated) {
    if hardware.HighSpeedPhysical() == regs::HSP_ULPI
        && hardware.FullSpeedPhysical() == regs::DEDICATED
    {
        //     LOG_DEBUG("HCD: ULPI FSLS configuration: enabled.\n");
        println!("HCD: ULPI FSLS configuration: enabled.");
        //     Core->Usb.UlpiFsls = true;
        //     Core->Usb.ulpi_clk_sus_m = true;
        //   } else {
        regs::Usb::modify(|usb| {
            usb.set_UlpiFsls(1);
            usb.set_ulpi_clk_sus_m(1);
        });
    } else {
        //     LOG_DEBUG("HCD: ULPI FSLS configuration: disabled.\n");
        println!("HCD: ULPI FSLS configuration: disabled.");
        //     Core->Usb.UlpiFsls = false;
        //     Core->Usb.ulpi_clk_sus_m = false;
        //   }
        regs::Usb::modify(|usb| {
            usb.set_UlpiFsls(0);
            usb.set_ulpi_clk_sus_m(0);
        });
    }
    //   WriteThroughReg(&Core->Usb);

    //
    //   LOG_DEBUG("HCD: DMA configuration: enabled.\n");
    println!("HCD: DMA configuration: enabled.");
    //   ReadBackReg(&Core->Ahb);
    //   Core->Ahb.DmaEnable = true;
    //   Core->Ahb.DmaRemainderMode = Incremental;
    //   WriteThroughReg(&Core->Ahb);
    regs::Ahb::modify(|ahb| {
        ahb.set_DmaEnable(1);
        ahb.set_DmaRemainderMode(regs::INCREMENTAL);
    });

    //   ReadBackReg(&Core->Usb);
    //   switch (Core->Hardware.OperatingMode) {
    match hardware.OperatingMode() {
        //     LOG_DEBUG("HCD: HNP/SRP configuration: HNP, SRP.\n");
        //     Core->Usb.HnpCapable = true;
        //     Core->Usb.SrpCapable = true;
        //     break;
        regs::HNP_SRP_CAPABLE => {
            println!("HCD: HNP/SRP configuration: HNP, SRP.");
            regs::Usb::modify(|usb| {
                usb.set_HnpCapable(1);
                usb.set_SrpCapable(1);
            });
        }
        //   case SRP_ONLY_CAPABLE:
        //   case SRP_CAPABLE_DEVICE:
        //   case SRP_CAPABLE_HOST:
        //     LOG_DEBUG("HCD: HNP/SRP configuration: SRP.\n");
        //     Core->Usb.HnpCapable = false;
        //     Core->Usb.SrpCapable = true;
        //     break;
        regs::SRP_ONLY_CAPABLE | regs::SRP_CAPABLE_DEVICE | regs::SRP_CAPABLE_HOST => {
            println!("HCD: HNP/SRP configuration: SRP.");
            regs::Usb::modify(|usb| {
                usb.set_HnpCapable(0);
                usb.set_SrpCapable(1);
            });
        }
        //   case NO_HNP_SRP_CAPABLE:
        //   case NO_SRP_CAPABLE_DEVICE:
        //   case NO_SRP_CAPABLE_HOST:
        //     LOG_DEBUG("HCD: HNP/SRP configuration: none.\n");
        //     Core->Usb.HnpCapable = false;
        //     Core->Usb.SrpCapable = false;
        //     break;
        //   }
        regs::NO_HNP_SRP_CAPABLE | regs::NO_SRP_CAPABLE_DEVICE | regs::NO_SRP_CAPABLE_HOST => {
            println!("HCD: HNP/SRP configuration: none.");
            regs::Usb::modify(|usb| {
                usb.set_HnpCapable(0);
                usb.set_SrpCapable(0);
            });
        }
        _ => {}
    }
    //   WriteThroughReg(&Core->Usb);
    //   LOG_DEBUG("HCD: Core started.\n");
    //   LOG_DEBUG("HCD: Starting host.\n");
    println!("HCD: Core started.");
    println!("HCD: Starting host.");

    //   ClearReg(Power);
    //   WriteThroughReg(Power);
    regs::PowerReg::clear();

    //   ReadBackReg(&Host->Config);
    //   if (Core->Hardware.HighSpeedPhysical == Ulpi
    //     && Core->Hardware.FullSpeedPhysical == Dedicated
    //     && Core->Usb.UlpiFsls) {
    let usb = regs::Usb::get();
    if hardware.HighSpeedPhysical() == regs::HSP_ULPI
        && hardware.FullSpeedPhysical() == regs::DEDICATED
        && usb.UlpiFsls() != 0
    {
        //     LOG_DEBUG("HCD: Host clock: 48Mhz.\n");
        //     Host->Config.ClockRate = Clock48MHz;
        println!("HCD: Host clock: 48Mhz.");
        regs::HostConfig::modify(|hc| hc.set_ClockRate(regs::CLOCK_48_MHZ));
        //   } else {
    } else {
        //     LOG_DEBUG("HCD: Host clock: 30-60Mhz.\n");
        //     Host->Config.ClockRate = Clock30_60MHz;
        //   }
        println!("HCD: Host clock: 30-60Mhz.");
        regs::HostConfig::modify(|hc| hc.set_ClockRate(regs::CLOCK_30_60_MHZ));
    }
    //   WriteThroughReg(&Host->Config);

    //   ReadBackReg(&Host->Config);
    //   Host->Config.FslsOnly = true;
    //   WriteThroughReg(&Host->Config);
    regs::HostConfig::modify(|hc| hc.set_FslsOnly(1));

    //   ReadBackReg(&Host->Config);
    let host_config = regs::HostConfig::get();
    let vendor_id = mmio_read(regs::HCD_DWC_VENDOR_ID);
    //   if (Host->Config.EnableDmaDescriptor ==
    //     Core->Hardware.DmaDescription &&
    //     (Core->VendorId & 0xfff) >= 0x90a) {
    if host_config.EnableDmaDescriptor() == hardware.DmaDescription() && vendor_id & 0xfff >= 0x90a
    {
        //     LOG_DEBUG("HCD: DMA descriptor: enabled.\n");
        println!("HCD: DMA descriptor: enabled.");
        //   } else {
    } else {
        //     LOG_DEBUG("HCD: DMA descriptor: disabled.\n");
        //   }
        println!("HCD: DMA descriptor: disabled.");
    }
    //   WriteThroughReg(&Host->Config);

    //   LOG_DEBUGF("HCD: FIFO configuration: Total=%#x Rx=%#x NPTx=%#x PTx=%#x.\n", ReceiveFifoSize + NonPeriodicFifoSize + PeriodicFifoSize, ReceiveFifoSize, NonPeriodicFifoSize, PeriodicFifoSize);
    println!(
        "HCD: FIFO configuration: Total=0x{:x} Rx=0x{:x} NPTx=0x{:x} PTx=0x{:x}.",
        regs::RECEIVE_FIFO_SIZE + regs::NON_PERIODIC_FIFO_SIZE + regs::PERIODIC_FIFO_SIZE,
        regs::RECEIVE_FIFO_SIZE,
        regs::NON_PERIODIC_FIFO_SIZE,
        regs::PERIODIC_FIFO_SIZE
    );

    //   ReadBackReg(&Core->Receive.Size);
    //   Core->Receive.Size = ReceiveFifoSize;
    //   WriteThroughReg(&Core->Receive.Size);
    mmio_write(regs::HCD_DWC_RECEIVE_SIZE, regs::RECEIVE_FIFO_SIZE);

    //   ReadBackReg(&Core->NonPeriodicFifo.Size);
    //   Core->NonPeriodicFifo.Size.Depth = NonPeriodicFifoSize;
    //   Core->NonPeriodicFifo.Size.StartAddress = ReceiveFifoSize;
    //   WriteThroughReg(&Core->NonPeriodicFifo.Size);

    {
        let mut fifo_size = regs::FifoSize { data: [0, 0, 0, 0] };
        fifo_size.set_Depth(regs::NON_PERIODIC_FIFO_SIZE);
        fifo_size.set_StartAddress(regs::RECEIVE_FIFO_SIZE);
        mmio_write(
            regs::HCD_DWC_NON_PERIODIC_FIFO_SIZE,
            u32::from_ne_bytes(fifo_size.data),
        );
    }

    //   ReadBackReg(&Core->PeriodicFifo.HostSize);
    //   Core->PeriodicFifo.HostSize.Depth = PeriodicFifoSize;
    //   Core->PeriodicFifo.HostSize.StartAddress = ReceiveFifoSize + NonPeriodicFifoSize;
    //   WriteThroughReg(&Core->PeriodicFifo.HostSize);
    {
        let mut fifo_size = regs::FifoSize { data: [0, 0, 0, 0] };
        fifo_size.set_Depth(regs::PERIODIC_FIFO_SIZE);
        fifo_size.set_StartAddress(regs::RECEIVE_FIFO_SIZE + regs::NON_PERIODIC_FIFO_SIZE);
        mmio_write(
            regs::HCD_DWC_PERIODIC_FIFO_SIZE,
            u32::from_ne_bytes(fifo_size.data),
        );
    }

    //   LOG_DEBUG("HCD: Set HNP: enabled.\n");
    println!("HCD: Set HNP: enabled.");

    //   ReadBackReg(&Core->OtgControl);
    //   Core->OtgControl.HostSetHnpEnable = true;
    //   WriteThroughReg(&Core->OtgControl);
    regs::OtgControl::modify(|otg_ctl| otg_ctl.set_HostSetHnpEnable(1));

    //   if ((result = HcdTransmitFifoFlush(FlushAll)) != OK)
    //     goto deallocate;
    hcd_transmit_fifo_flush(regs::FLUSH_ALL)?;
    //   if ((result = HcdReceiveFifoFlush()) != OK)
    //     goto deallocate;
    hcd_receive_fifo_flush()?;

    //   if (!Host->Config.EnableDmaDescriptor) {
    if host_config.EnableDmaDescriptor() == 0 {
        //     for (u32 channel = 0; channel < Core->Hardware.HostChannelCount; channel++) {
        for channel in 0..hardware.HostChannelCount() {
            //       ReadBackReg(&Host->Channel[channel].Characteristic);
            //       Host->Channel[channel].Characteristic.Enable = false;
            //       Host->Channel[channel].Characteristic.Disable = true;
            //       Host->Channel[channel].Characteristic.EndPointDirection = In;
            //       WriteThroughReg(&Host->Channel[channel].Characteristic);
            regs::HostChannelCharacteristic::modify(channel, |ch| {
                ch.set_Enable(0);
                ch.set_Disable(1);
                ch.set_EndPointDirection(regs::EP_DIR_IN);
            })
        }

        //     // Halt channels to put them into known state.
        //     for (u32 channel = 0; channel < Core->Hardware.HostChannelCount; channel++) {
        for channel in 0..hardware.HostChannelCount() {
            //       ReadBackReg(&Host->Channel[channel].Characteristic);
            //       Host->Channel[channel].Characteristic.Enable = true;
            //       Host->Channel[channel].Characteristic.Disable = true;
            //       Host->Channel[channel].Characteristic.EndPointDirection = In;
            //       WriteThroughReg(&Host->Channel[channel].Characteristic);
            regs::HostChannelCharacteristic::modify(channel, |ch| {
                ch.set_Enable(1);
                ch.set_Disable(1);
                ch.set_EndPointDirection(regs::EP_DIR_IN);
            });

            //       timeout = 0;
            //       do {
            //         ReadBackReg(&Host->Channel[channel].Characteristic);
            //
            //         if (timeout++ > 0x100000) {
            //           LOGF("HCD: Unable to clear halt on channel %u.\n", channel);
            //         }
            //       } while (Host->Channel[channel].Characteristic.Enable);
            //     }
            //   }
            let mut got_clear: bool = false;
            for _ in 0..0x100000 {
                if regs::HostChannelCharacteristic::get(channel).Enable() == 0 {
                    got_clear = true;
                    break;
                }
            }
            if !got_clear {
                println!(
                    "[WARN] USB: HCD: Unable to clear halt on channel {}",
                    channel
                );
            }
        }
    }

    //   ReadBackReg(&Host->Port);
    let mut host_port = regs::HostPort::get();
    //   if (!Host->Port.Power) {
    if host_port.Power() == 0 {
        //     LOG_DEBUG("HCD: Powering up port.\n");
        println!("HCD: Powering up port.");
        host_port.set_Power(1);
        host_port.set();
        //     Host->Port.Power = true;
        //     WriteThroughRegMask(&Host->Port, 0x1000);
        //   }
    }

    //   LOG_DEBUG("HCD: Reset port.\n");
    println!("HCD: Reset port.");

    //   ReadBackReg(&Host->Port);
    //   Host->Port.Reset = true;
    //   WriteThroughRegMask(&Host->Port, 0x100);
    regs::HostPort::modify(|hp| hp.set_Reset(1));

    //   MicroDelay(50000);
    delay_us_sync(50000);

    //   Host->Port.Reset = false;
    //   WriteThroughRegMask(&Host->Port, 0x100);
    //   ReadBackReg(&Host->Port);
    regs::HostPort::modify(|hp| hp.set_Reset(0));

    //   LOG_DEBUG("HCD: Successfully started.\n");
    println!("HCD: Successfully started.");

    Ok(())
}
