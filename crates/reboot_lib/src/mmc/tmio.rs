use crate::{ARM7Interrupt, ClockCnt, MMC, MMC_CONTROLLER, StorageSector, TMIOPort, disable_interrupt, enable_interrupt, interupts, mmc::Command, set_interrupt_function, swi_delay, swi_halt};

use super::{Control, DataControl32, Status};


impl MMC {
    pub unsafe fn initialize_sdmmc(&self) {
        disable_interrupt(ARM7Interrupt::SDMMC);
        set_interrupt_function(ARM7Interrupt::SDMMC, sdmmc_irq);
        
        self.data_control_32.write(DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32);
        self.block_len_32.write(512);
        self.block_count_32.write(1);
        self.data_control.write(Control::USE_DATA32);

        self.soft_reset.write(0);
        self.soft_reset.write(1);
        
        self.port_select.write(0);
        self.block_count.write(1);
        self.irmask.write(Status::TX_REQUEST | Status::RX_READY | Status::DAT3_INSERT | Status::DAT3_REMOVE | Status::UNKNOWN);
        self.clock_control.write(ClockCnt::FREQ_262K);
        self.block_len.write(512);

        self.options.write((1<<14) | (1<<15) | (11<<4) | 8);
        self.ext_card_detect_mask.write(0xFFFF);
        self.ext_card_detect_dat3_mask.write(0xFFFF);
        
        
        self.sdio_mode.write(0);
        self.sdio_mask.write(0xFFFF);
        self.ext_sdio_irq.write((1<<10) | (1<<9) | (1<<8));

        enable_interrupt(ARM7Interrupt::SDMMC);
    }
    pub unsafe fn powerup_port(&self, port: &mut TMIOPort) {
        self.clock_control.write(ClockCnt::ENABLE | ClockCnt::FREQ_262K);
        self.port_select.write(port.port_num);
        crate::swi::swi_delay(0x1200);
    }
    pub unsafe fn set_port(&self, port: &mut TMIOPort) {
        self.port_select.write(port.port_num);
        self.clock_control.write(port.clock);
        self.block_len.write(port.block_len);
        self.options.write(port.option);
        self.block_len_32.write(port.block_len as _);
        self.block_count.write(port.buffer.len() as _);
        
    }
    pub unsafe fn card_inserted(&self) -> bool {
        self.status.read().contains(Status::DETECTED)
    }
    pub unsafe fn card_writable(&self) -> bool {
        self.status.read().contains(Status::WRITEABLE)
    }
    pub unsafe fn new_send_command(&self, port: &mut TMIOPort, command: Command, argument: u32) -> Status {
        self.set_port(port);
        self.stop_action.write(1<<8);
        self.param.write(argument);

        let Some(buf) = port.buffer.as_mut() else {return Status::ERR_ILLEGAL_ACCESS};
        let control = if command.reads_data() {
            DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32 | DataControl32::ENABLE_RX_IRQ
        } else {
            DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32 | DataControl32::ENABLE_TX_IRQ
        };
        self.data_control_32.write(control);
        
        let mut timeout = 0;
        while self
            .data_control_32
            .read()
            .intersects(DataControl32::RX_READY)
        {
            timeout += 1;
            if timeout > 0x10_0000 {
                return !Status::INSERTED;
            }
        }

        core::ptr::write_volatile(&mut MMC_STATUS, Status::empty());
        self.status.write(Status::empty());
        self.command.write(command as _);


        if command.transmits_data() {
            let Some(buf) = port.buffer.as_mut() else {
                return Status::from_bits_retain(0xB4B4B4B4);
            };
            let mut sector_iter = buf.iter_mut();
            let Some(mut current_sector) = sector_iter.next() else {
                return Status::from_bits_retain(0x4B4B4B4B);
            };
            timeout = 0;
            if command.reads_data() {
                // Read loop
                while !self.status.read().intersects(Status::ALL_ERRORS) {
                    timeout += 1;
                    if timeout > 0x10_0000 {
                        return !Status::INSERTED;
                    }
                    if self
                        .data_control_32
                        .read()
                        .contains(DataControl32::RX_READY)
                    {
                        timeout = 0;
                        //self.status.write(!Status::RX_READY);
                        for (i, word) in current_sector.0.iter_mut().enumerate() {
                            (word as *mut u32).write_volatile(self.data_fifo_32.read());
                        }
                        let Some(next_sector) = sector_iter.next() else {
                            break;
                        };
                        current_sector = next_sector;
                    }
                }
            } else {
                // Write loop
                while !self.status.read().intersects(Status::ALL_ERRORS) {
                    timeout += 1;
                    if timeout > 0x10_0000 {
                        return !Status::INSERTED;
                    }
                    if !self.data_control_32.read().contains(DataControl32::TX_BUSY) {
                        timeout = 0;
                        for (i, word) in current_sector.0.iter_mut().enumerate() {
                            self.data_fifo_32.write(*word);
                        }
                        let Some(next_sector) = sector_iter.next() else {
                            break;
                        };
                        current_sector = next_sector;
                    }
                }
            }
        }
        while !core::ptr::read_volatile(&MMC_STATUS).contains(Status::RESPONSE_END) {
            swi_halt();
        }

        if (command as u16) & (7 << 8) != (6 << 8) {
            port.response[0] = self.response[0].read();
        } else {
            for i in 0..4 {
                port.response[i] = self.response[i].read()
            }
        }

        core::ptr::read_volatile(&MMC_STATUS) & Status::ALL_ERRORS
    }
}
static mut MMC_STATUS: Status = Status::EMPTY;
unsafe fn sdmmc_irq() {
    MMC_STATUS |= MMC_CONTROLLER.status.read();
    MMC_CONTROLLER.status.write(Status::CMD_BUSY);
}