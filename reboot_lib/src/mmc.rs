use core::ops::{BitAndAssign, BitOrAssign, Not};

use crate::MemoryWrapper;
use volatile_register::*;

pub mod driver;
pub mod new_driver;
pub mod newer_driver;
pub mod tmio;

pub const MMC_CONTROLLER: MemoryWrapper<MMC> = MemoryWrapper(0x4004800 as *mut MMC);
pub const SDIO_CONTROLLER: MemoryWrapper<MMC> = MemoryWrapper(0x4004A00 as *mut MMC);

const TMIO_STAT1_CMD_IDX_ERR: u16 = 0x0001;
const TMIO_STAT1_CRCFAIL: u16 = 0x0002;
const TMIO_STAT1_STOPBIT_ERR: u16 = 0x0004;
const TMIO_STAT1_DATATIMEOUT: u16 = 0x0008;
const TMIO_STAT1_RXOVERFLOW: u16 = 0x0010;
const TMIO_STAT1_TXUNDERRUN: u16 = 0x0020;
const TMIO_STAT1_CMDTIMEOUT: u16 = 0x0040;
const TMIO_STAT1_RXRDY: u16 = 0x0100;
const TMIO_STAT1_TXRQ: u16 = 0x0200;
const TMIO_STAT1_ILL_FUNC: u16 = 0x2000;
const TMIO_STAT1_CMD_BUSY: u16 = 0x4000;
const TMIO_STAT1_ILL_ACCESS: u16 = 0x8000;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ClockCnt: u16 {
        const ENABLE = (1<<8);
        const AUTO_STOP = (1<<9);

        const FREQ_65K = (0x80 >> 0);
        const FREQ_131K = (0x80 >> 1);
        const FREQ_262K = (0x80 >> 2);
        const FREQ_523K = (0x80 >> 3);
        const FREQ_1M = (0x80 >> 4);
        const FREQ_2M = (0x80 >> 5);
        const FREQ_4M = (0x80 >> 6);
        const FREQ_8M = (0x80 >> 7);
        const FREQ_16M = (0x80 >> 8);
    }
}

const TMIO_MASK_GW: u16 = (TMIO_STAT1_ILL_ACCESS
    | TMIO_STAT1_CMDTIMEOUT
    | TMIO_STAT1_TXUNDERRUN
    | TMIO_STAT1_RXOVERFLOW
    | TMIO_STAT1_DATATIMEOUT
    | TMIO_STAT1_STOPBIT_ERR
    | TMIO_STAT1_CRCFAIL
    | TMIO_STAT1_CMD_IDX_ERR);

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Control: u16 {
        const USE_DATA32 = (1 << 1);
        const UNKNOWN_SET = (1 << 4) | (1<<12);
        const MASK = !((1<<5) | (1<<1));
    }

    #[derive(Clone, Copy, PartialEq)]
    pub struct Status: u32 {
        const EMPTY = 0;
        const RESPONSE_END = (1 << 0);
        const DATA_END = (1 << 2);
        const REMOVE = (1 << 3);
        const INSERTED = (1 << 4);
        const DETECTED = (1 << 5);
        const WRITEABLE = (1 << 7);
        const DAT3_REMOVE = (1<<8);
        const DAT3_INSERT = (1<<9);
        const DAT3_DETECT = (1<<10);

        const ERR_CMD_INDEX = (1<<16);
        const ERR_BAD_CRC = (1<<17);
        const ERR_STOP = (1<<18);
        const ERR_DATA_TIMEOUT = (1<<19);
        const ERR_RX_OVERFLOW = (1<<20);
        const ERR_TX_UNDERFLOW = (1<<21);
        const ERR_CMD_TIMEOUT = (1<<22);


        const SD_BUSY = (1<<23);
        const RX_READY = (1<<24);
        const TX_REQUEST = (1<<25);

        const UNKNOWN = (1<<27);



        const CMD_BUSY = (1<<30);
        const ERR_ILLEGAL_ACCESS = (1<<31);

        const ALL_ERRORS = (1 << 31) | (0b_111_1111<<16);
    }
}
impl Status {
    pub fn successful(&self) -> bool {
        self.intersection(Status::ALL_ERRORS).is_empty()
    }
}
#[repr(C)]
pub struct MMC {
    pub command: RW<u16>,
    pub port_select: RW<u16>,
    pub param: RW<u32>,
    pub stop_action: RW<u16>,
    pub block_count: RW<u16>,
    pub response: [RO<u32>; 4],
    pub status: RW<Status>,
    pub irmask: RW<Status>,
    pub clock_control: RW<ClockCnt>,
    pub block_len: RW<u16>,
    pub options: RW<u16>,
    _unused: [u16; 1],
    pub error_info: RW<u32>,
    pub data_fifo: RW<u16>,
    _0x32: u16,
    pub sdio_mode: RW<u16>,
    pub sdio_status: RW<u16>,
    pub sdio_mask: RW<u16>,
    _0x3a: [u16; 79],
    pub data_control: RW<Control>,
    _0xda: [u16; 3],
    pub soft_reset: RW<u16>,
    pub revision: RW<u16>,
    _0xe4: [u16; 7],
    pub unknownf: RW<u16>,
    pub ext_sdio_irq: RW<u16>,
    pub ext_write_protect: RW<u16>,
    pub ext_card_detect: RW<u16>,
    pub ext_card_detect_dat3: RW<u16>,
    pub ext_card_detect_mask: RW<u16>,
    pub ext_card_detect_dat3_mask: RW<u16>,
    pub data_control_32: RW<DataControl32>,
    _0x102: u16,
    pub block_len_32: RW<u16>,
    _0x106: u16,
    pub block_count_32: RW<u16>,
    _0x10a: u16,
    pub data_fifo_32: RW<u32>,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct DataControl32: u16 {
        const ENABLE_RX_IRQ = (1 << 11);
        const ENABLE_TX_IRQ = (1 << 12);
        const CLEAR_FIFO_32 = (1 << 10);
        const RX_READY = (1 << 8);
        const TX_READY = (1 << 9);
        const USE_DATA32 = (1 << 1);
    }
}
// A rust implementation of profi2000's TMIO in pure rust. And awful.

impl MMC {
    pub unsafe fn tmio_init(&self) {
        self.data_control_32
            .write(DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32); // enable and clear data32 fifo
        self.block_len_32.write(512);
        self.block_count_32.write(1);
        self.data_control.write(Control::USE_DATA32); // enable DMA requests? (gbatek says data32 mode?)

        //reset and un-reset
        self.soft_reset.write(0);
        self.soft_reset.write(1);

        self.port_select.write(0);
        self.block_count.write(1);
        self.irmask.write(
            Status::UNKNOWN
                | Status::TX_REQUEST
                | Status::RX_READY
                | Status::DAT3_INSERT
                | Status::DAT3_REMOVE,
        );
        self.clock_control.write(ClockCnt::FREQ_262K);
        self.block_len.write(512);
        self.options.write((1 << 15) | (1 << 14) | ((11 << 4) | 8));
        self.ext_card_detect_mask.write(0xFFFF);
        self.ext_card_detect_dat3_mask.write(0xFFFF);

        //disable SDIO
        self.sdio_mode.write(0);
        self.sdio_mask.write(0xFFFF);
        self.ext_sdio_irq.write((1 << 10) | (1 << 9) | (1 << 8));
    }
    unsafe fn tmio_set_port(&self, port: &TMIOPort) {
        self.port_select.write(port.port_num as u16);
        self.clock_control.write(port.clock);
        self.block_len.write(port.block_len);
        self.options.write(port.option);
        self.block_len_32.write(port.block_len);
    }
    pub fn tmio_card_detected(&self) -> bool {
        self.status.read().contains(Status::DETECTED)
    }
    pub fn tmio_card_writable(&self) -> bool {
        self.status.read().contains(Status::WRITEABLE)
    }
    unsafe fn tmio_powerup(&self, port: &mut TMIOPort) {
        port.clock = ClockCnt::FREQ_262K | ClockCnt::ENABLE;
        self.tmio_set_port(port);
        crate::swi::swi_delay(0x900);
    }
    unsafe fn tmio_get_response(&self, port: &mut TMIOPort, cmd: u16) {
        if cmd & (7 << 8) != (6 << 8) {
            port.response[0] = self.response[0].read();
        } else {
            for i in 0..4 {
                port.response[i] = self.response[i].read()
            }
        }
    }
    pub unsafe fn send_command(
        &self,
        port: &mut TMIOPort,
        command: Command,
        argument: u32,
    ) -> Status {
        let command = command as u16;
        let mut status = Status::empty();

        let flags = if command & (1 << 11) > 0 {
            Status::DATA_END
        } else {
            Status::empty()
        };
        self.tmio_set_port(port);

        self.irmask.write(Status::empty());
        self.status.write(Status::empty());

        self.block_count.write(port.buffer.len() as u16);
        self.stop_action.write(1 << 8);

        self.data_control_32.modify(|f| {
            (f & !(DataControl32::ENABLE_RX_IRQ | DataControl32::ENABLE_TX_IRQ))
                | DataControl32::CLEAR_FIFO_32
                | DataControl32::USE_DATA32
        });
        self.block_len_32.write(port.block_len);
        self.param.write(argument);
        let cmd = command;
        self.command.write(cmd);

        let (mut ptr, mut len) = port.buffer.to_raw_parts();
        let use_buf = !ptr.is_null();
        loop {
            let control = self.data_control_32.read();
            status = self.status.read();
            if use_buf {
                let word_count = (port.block_len >> 2);
                if control.contains(DataControl32::RX_READY) {
                    for i in 0..word_count {
                        (ptr as *mut u32)
                            .add(i as usize)
                            .write_volatile(self.data_fifo_32.read());
                    }
                }
                if control.contains(DataControl32::TX_READY) {
                    //what now? (Write)
                }
                len -= 1;
                ptr.add(word_count as usize);
            }

            if status.contains(Status::ALL_ERRORS) {
                break;
            }
            if !status.intersects(Status::CMD_BUSY) {
                if len == 0 {
                    break;
                }
                if status.contains(flags) {
                    break;
                }
            }
        }
        let resp = status.intersection(Status::ALL_ERRORS);
        self.status.write(Status::empty());
        self.tmio_get_response(port, cmd);
        return resp;

        let buf = port.buffer;

        while !status.intersects(Status::RESPONSE_END) {
            status |= self.status.read();
        }
        self.tmio_get_response(port, command);
        if command & CMD_DATA_EN > 0 {
            if !buf.is_null() {
                self.cpu_transfer(command, buf, &mut status);
            }
            while !status.intersects(Status::DATA_END) {
                status |= self.status.read();
            }
        }
        while self.status.read().contains(Status::CMD_BUSY) {}
        status |= self.status.read();
        status.intersection(Status::ALL_ERRORS)
    }
    unsafe fn cpu_transfer(
        &self,
        command: u16,
        buf: *mut [crate::StorageSector],
        status: &mut Status,
    ) {
        if command & CMD_DATA_R > 0 {
            *status |= self.status.read();
            for sector in (&mut *buf).iter_mut() {
                if !status.intersects(Status::ALL_ERRORS) {
                    while self
                        .data_control_32
                        .read()
                        .intersection(DataControl32::RX_READY)
                        .is_empty()
                    {}
                    for word in &mut sector.0 {
                        *word = self.data_fifo_32.read()
                    }
                }
            }
        } else {
            *status |= self.status.read();
            for sector in (&*buf).iter() {
                if !status.intersects(Status::ALL_ERRORS) {
                    while self
                        .data_control_32
                        .read()
                        .contains(DataControl32::TX_READY)
                    {}
                    for word in &sector.0 {
                        self.data_fifo_32.write(*word)
                    }
                }
            }
        }
    }
}
const fn none(command_number: u16) -> u16 {
    command_number | CMD_RESP_NONE
}
const fn r1(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1
}
const fn r1b(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1B
}
const fn r2(command_number: u16) -> u16 {
    command_number | CMD_RESP_R2
}
const fn r3(command_number: u16) -> u16 {
    command_number | CMD_RESP_R3
}
const fn r4(command_number: u16) -> u16 {
    command_number | CMD_RESP_R4
}
const fn r5(command_number: u16) -> u16 {
    command_number | CMD_RESP_R5
}
const fn r6(command_number: u16) -> u16 {
    command_number | CMD_RESP_R6
}
const fn r7(command_number: u16) -> u16 {
    command_number | CMD_RESP_R7
}
const fn r1_r(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1 | CMD_DATA_EN | CMD_DATA_R
}
const fn r1_w(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1 | CMD_DATA_EN | CMD_DATA_W
}

const fn acmd_r1(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1 | (1 << 6)
}
const fn acmd_r3(command_number: u16) -> u16 {
    command_number | CMD_RESP_R3 | (1 << 6)
}
const fn acmd_r1_r(command_number: u16) -> u16 {
    command_number | CMD_RESP_R1 | CMD_DATA_EN | CMD_DATA_R | (1 << 6)
}
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum Command {
    //basic commands (class 0)
    GoIdleState = none(0),
    AllSendCID = r2(2),
    SetSendRelativeAddr = r1(3),
    SetDSR = none(4),
    SelectCard = r1b(7),
    DeselectCard = none(7),
    SendIfCondition = r7(8),
    SendCSD = r2(9),
    SendCID = r2(10),
    VoltageSwitch = r1(11),
    StopTransmission = r1b(12),
    SendStatus = r1(13),
    GoInactiveState = none(15),

    //block oriented commands
    SetBlockLen = r1(16),
    ReadSingleBlock = r1_r(17),
    ReadMutliBlocks = r1_r(18) | CMD_DATA_MULTI,
    SendTuningBlock = r1_r(19),
    SpeedClassControl = r1b(20),
    AddressExtension = r1(22),
    SetBlockCount = r1(23),

    WriteSingleBlock = r1_w(24),
    WriteMultiBlocks = r1_w(25),
    ProgramCSD = r1_w(27),

    SetWriteProtection = r1b(28),
    ClearWriteProtection = r1b(29),
    SendWriteProtection = r1_r(30),

    EraseWriteBlockStart = r1(32), //  R1, [31:0] data address.
    EraseWriteBlockEnd = r1(33),   //  R1, [31:0] data address.
    Erase = r1b(38),               // R1b, [31:0] Erase Function.

    LockUnlock = r1_w(42), //  R1, [31:0] Reserved bits (Set all 0).

    AppCommand = r1(55),            //  R1, [31:16] RCA [15:0] stuff bits.
    GenericCommandRead = r1_r(56),  //  R1, [31:1] stuff bits. [0]: RD/WR = 1.
    GenertiCommandWrite = r1_w(56), //  R1, [31:1] stuff bits. [0]: RD/WR = 0.

    AppSetBusWidth = acmd_r1(6), //  R1, [31:2] stuff bits [1:0] bus width.
    AppSDStatus = acmd_r1_r(13), //  R1, [31:0] stuff bits.
    AppSendNumWrBlocks = acmd_r1_r(22), //  R1, [31:0] stuff bits.
    AppSetWrBlockEraseCount = acmd_r1(23), //  R1, [31:23] stuff bits [22:0] Number of blocks.
    AppSendOpCondition = acmd_r3(41), //  R3, [31] reserved bit [30] HCS (OCR[30]) [29] reserved for eSD [28] XPC [27:25] reserved bits [24] S18R [23:0] VDD Voltage Window (OCR[23:0]).
    AppSetClearCardSelect = acmd_r1(42), //  R1, [31:1] stuff bits [0] set_cd.
    AppSendSCR = acmd_r1_r(51),       //  R1, [31:0] stuff bits.

    SwitchFunction = r1_r(6), //  R1, [31] Mode 0: Check function 1: Switch function [30:24] reserved (All '0') [23:20] reserved for function group 6 (0h or Fh) [19:16] reserved for function group 5 (0h or Fh) [15:12] function group 4 for PowerLimit [11:8] function group 3 for Drive Strength [7:4] function group 2 for Command System [3:0] function group 1 for Access Mode.

    ReadExtensionSingle = r1_r(48), //  R1, [31] MIO0: Memory, 1: I/O [30:27] FNO[26] Reserved (=0) [25:9] ADDR [8:0] LEN.
    WriteExtensionSingle = r1_w(49), //  R1, [31] MIO0: Memory, 1: I/O [30:27] FNO [26] MW [25:9] ADDR [8:0] LEN/MASK.
    ReadExtensionMultiple = r1_r(58), //  R1, [31] MIO0: Memory, 1: I/O [30:27] FNO [26] BUS0: 512B, 1: 32KB [25:9] ADDR [8:0] BUC.
    WriteExtensionMultiple = r1_w(59), //  R1, [31] MIO0: Memory, 1: I/O [30:27] FNO [26] BUS0: 512B, 1: 32KB [25:9] ADDR [8:0] BUC.

    QueueManagement = r1b(43), // R1b, [31:21] Reserved [20:16]: Task ID [3:0]: Operation Code (Abort tasks etc.).
    QueueTaskInfoA = r1(44), //  R1, [31] Reserved [30] Direction [29:24] Extended Address [23] Priority [22:21] Reserved [20:16] Task ID [15:0] Number of Blocks.
    QueueTaskInfoB = r1(45), //  R1, [31:0] Start block address.
    QueueReadTask = r1_r(46), //  R1, [31:21] Reserved [20:16] Task ID [15:0] Reserved.
    QueueWriteTask = r1_w(47), //  R1, [31:21] Reserved [20:16] Task ID [15:0] Reserved.

    MMCSendOptionalCondition = r3(1),
}

impl Command {
    pub fn transmits_data(&self) -> bool {
        *self as u16 & CMD_DATA_EN > 0
    }
    pub fn reads_data(&self) -> bool {
        *self as u16 & CMD_DATA_R > 0
    }
}

const CMD_RESP_AUTO: u16 = 0; // Response type auto. Only works with certain commands.
const CMD_RESP_NONE: u16 = 3 << 8; // Response type none.
const CMD_RESP_R1: u16 = 4 << 8; // Response type R1 48 bit.
const CMD_RESP_R5: u16 = CMD_RESP_R1; // Response type R5 48 bit.
const CMD_RESP_R6: u16 = CMD_RESP_R1; // Response type R6 48 bit.
const CMD_RESP_R7: u16 = CMD_RESP_R1; // Response type R7 48 bit.
const CMD_RESP_R1B: u16 = 5 << 8; // Response type R1b 48 bit + busy.
const CMD_RESP_R5B: u16 = CMD_RESP_R1B; // Response type R5b 48 bit + busy.
const CMD_RESP_R2: u16 = 6 << 8; // Response type R2 136 bit.
const CMD_RESP_R3: u16 = 7 << 8; // Response type R3 48 bit OCR without CRC.
const CMD_RESP_R4: u16 = CMD_RESP_R3; // Response type R4 48 bit OCR without CRC.
const CMD_RESP_MASK: u16 = CMD_RESP_R3;

const CMD_DATA_EN: u16 = 1 << 11;
const CMD_DATA_R: u16 = 1 << 12;
const CMD_DATA_MULTI: u16 = 1 << 13;
const CMD_DATA_W: u16 = 0;

#[derive(Debug)]
pub struct TMIOPort {
    pub port_num: u8,
    pub clock: ClockCnt,
    pub block_len: u16,
    pub option: u16,
    pub buffer: *mut [crate::StorageSector],
    pub response: [u32; 4],
}
impl TMIOPort {
    pub const fn init(port_num: u8) -> Self {
        Self {
            port_num,
            clock: ClockCnt::FREQ_262K,
            block_len: 512,
            option: (1 << 15) | (1 << 14) | ((11 << 4) | 8),
            buffer: unsafe { core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0) },
            response: [0; 4],
        }
    }
}
impl Default for TMIOPort {
    fn default() -> Self {
        Self {
            port_num: Default::default(),
            clock: ClockCnt::empty(),
            block_len: Default::default(),
            option: Default::default(),
            buffer: unsafe { core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0) },
            response: Default::default(),
        }
    }
}
