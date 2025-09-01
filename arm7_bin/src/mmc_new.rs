use core::ptr::read_volatile;

use reboot_lib::{
    swi_halt, ClockCnt, DataControl32, Status, StorageSector, MMC, MMC_CONTROLLER, SDIO_CONTROLLER,
};

pub trait MMCCommand: SDMMCCommand {}
pub trait SDCommand: SDMMCCommand {}
pub trait SDIOCommand: SDMMCCommand {}
pub trait SDMMCCommand {
    const COMMAND_INDEX: CommandIndex;
    type Response: SDMMCResponse;
    type Argument: SDMMCArgument;
    fn buffer(&self) -> BufferType {
        BufferType::None
    }
}
pub enum BufferType {
    None,
    Read(*mut [StorageSector]),
    Write(*const [StorageSector]),
}
pub trait SDMMCArgument {
    fn as_argument(self) -> CommandArgument;
}
pub trait SDMMCResponse {
    fn from_response(resp: [u32; 4]) -> Self;
}
pub struct CommandIndex(u16);
pub struct CommandArgument(u32);

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq)]
    pub struct SDMMCError: u32 {
        const CMD_INDEX = (1<<16);
        const BAD_CRC = (1<<17);
        const STOP = (1<<18);
        const DATA_TIMEOUT = (1<<19);
        const RX_OVERFLOW = (1<<20);
        const TX_UNDERFLOW = (1<<21);
        const CMD_TIMEOUT = (1<<22);
    }
}

pub struct Port<const id: u8> {
    clock_ctrl: ClockCnt,
    option: u16,
    block_len: u16,
}
impl Port<0> {
    fn send_command<C: SDCommand>(&self, command: C, argument: C::Argument)-> Result<C::Response, SDMMCError> {
        send_command(self, command, argument)
    }
}
impl Port<1> {
    fn send_command<C: MMCCommand>(&self, command: C, argument: C::Argument)-> Result<C::Response, SDMMCError> {
        send_command(self, command, argument)
    }
}
impl Port<2> {
    fn send_command<C: SDIOCommand>(&self, command: C, argument: C::Argument)-> Result<C::Response, SDMMCError> {
        send_command(self, command, argument)
    }
}
fn send_command<C: SDMMCCommand, const port_num: u8>(
    port: &Port<port_num>,
    command: C,
    arg: C::Argument,
) -> Result<C::Response, SDMMCError> {
    let Port { clock_ctrl, option, block_len } = port;
    let (controller, status) = unsafe {
        match port_num > 1 {
            true => (&*MMC_CONTROLLER, &mut SDMMC_STATUS),
            false => (&*SDIO_CONTROLLER, &mut SDIO_STATUS),
        }
    };
    let status = unsafe {
        raw_send_sdmmc_command(
            port_num,
            *clock_ctrl,
            *option,
            *block_len,
            C::COMMAND_INDEX,
            arg.as_argument(),
            command.buffer(),
            controller,
            status,
        )
    };
    match status {
        Status::EMPTY => Ok(C::Response::from_response(core::array::from_fn(|i| {
            controller.response[i].read()
        }))),
        error => Err(SDMMCError::from_bits_retain(error.bits())),
    }
}

static mut SDMMC_STATUS: Status = Status::empty();
static mut SDIO_STATUS: Status = Status::empty();

unsafe fn tmio_mmc_irq() {
    //update our status copy
    SDMMC_STATUS |= MMC_CONTROLLER.status.read();
    //acknowledge all irq's except CMD_BUSY (it disables itself)
    MMC_CONTROLLER.status.write(Status::CMD_BUSY);
}

/// sends a raw sdmmc command, with no abstractions
unsafe fn raw_send_sdmmc_command(
    port_num: u8,
    clock_setting: ClockCnt,
    option: u16,
    block_length: u16,
    cmd_index: CommandIndex,
    argument: CommandArgument,
    buffer: BufferType,
    controller: &MMC,
    status: *mut Status,
) -> Status {
    let MMC {
        command,
        port_select,
        param,
        stop_action,
        block_count,
        clock_control,
        block_len,
        options,
        data_control_32,
        block_len_32,
        ..
    } = controller;
    status.write_volatile(Status::empty());
    // Setup Port

    port_select.write((port_num & 1) as u16);
    clock_control.write(clock_setting);
    options.write(option);
    block_len.write(block_length);
    block_len_32.write(block_length);

    // Setup block control registers
    let (b_c, control) = match &buffer {
        BufferType::None => (0, DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32),
        BufferType::Read(storage_sectors) => (
            storage_sectors.len(),
            DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32 | DataControl32::ENABLE_RX_IRQ,
        ),
        BufferType::Write(storage_sectors) => (
            storage_sectors.len(),
            DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32 | DataControl32::ENABLE_TX_IRQ,
        ),
    };
    block_count.write(b_c as u16);
    stop_action.write(1 << 8); //AUTO_STOP
    param.write(argument.0);
    data_control_32.write(control);

    // start command
    command.write(cmd_index.0);
    // Wait for response
    while !status.read_volatile().contains(Status::RESPONSE_END) {
        swi_halt();
    }
    // Read/Write data if it's a command that does so
    match buffer {
        BufferType::None => (),
        BufferType::Read(buffer) => {
            if let Some(buffer) = buffer.as_mut() {
                cpu_read(buffer, controller, status)
            } else {
                ndma_readwrite(status);
            }
        }
        BufferType::Write(buffer) => {
            if let Some(buffer) = buffer.as_ref() {
                cpu_write(buffer, controller, status);
            } else {
                ndma_readwrite(status);
            }
        }
    }
    // Finished
    status.read_volatile().intersection(Status::ALL_ERRORS)
}

unsafe fn ndma_readwrite(status: *const Status) {
    while status.read_volatile().contains(Status::DATA_END) {
        swi_halt();
    }
}
unsafe fn cpu_read(buffer: &mut [StorageSector], controller: &MMC, status: *const Status) {
    let mut sectors = buffer.iter_mut();
    let Some(mut current_block) = sectors.next() else {
        return;
    };
    while !status.read_volatile().intersects(Status::ALL_ERRORS) {
        if status.read_volatile().contains(Status::RX_READY) {
            for word in AsMut::<[u32]>::as_mut(current_block) {
                *word = controller.data_fifo_32.read();
            }
            let Some(next_block) = sectors.next() else {
                return;
            };
            current_block = next_block;
        } else {
            swi_halt();
        }
    }
}
unsafe fn cpu_write(buffer: &[StorageSector], controller: &MMC, status: *const Status) {
    let mut sectors = buffer.iter();
    let Some(mut current_block) = sectors.next() else {
        return;
    };
    while !status.read_volatile().intersects(Status::ALL_ERRORS) {
        if status.read_volatile().contains(Status::TX_REQUEST) {
            for word in AsRef::<[u32]>::as_ref(current_block) {
                controller.data_fifo_32.write(*word);
            }
            let Some(next_block) = sectors.next() else {
                return;
            };
            current_block = next_block;
        } else {
            swi_halt();
        }
    }
}
