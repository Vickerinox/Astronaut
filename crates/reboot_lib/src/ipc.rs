use crate::MemoryWrapper;
use alloc::alloc::Global;
use bitflags::bitflags;
use volatile_register::*;
pub const IPC_FIFO_HARDWARE: MemoryWrapper<IPCFifoHardware> =
    MemoryWrapper(0x4000180 as *mut IPCFifoHardware);
const IPC_FIFO_RECIEVE: MemoryWrapper<RO<u32>> = MemoryWrapper(0x4100000 as *mut RO<u32>);

#[repr(C)]
pub struct IPCFifoHardware {
    pub sync: RW<IPCSYNC>,
    pub control: RW<IPCCNT>,
    pub send: WO<u32>,
}
#[derive(Debug)]
pub enum SendFifoError {
    QueueFull,
    FifoDisabled,
}
#[derive(Debug)]
pub enum RecieveFifoError {
    QueueEmpty,
    FifoDisabled,
}
impl IPCFifoHardware {
    pub unsafe fn recv_fifo_empty(&self) -> bool {
        self.control.read().contains(IPCCNT::RECV_FIFO_EMPTY)
    }
    pub unsafe fn enable(&self) {
        self.control
            .write(IPCCNT::ENABLE_FIFOS | IPCCNT::FLUSH_SEND_FIFO);
    }
    pub unsafe fn set_status(&self, status: u8) {
        let bits = ((status & 0xF) as u32) << 8;
        self.sync.write(IPCSYNC::from_bits_retain(bits));
    }
    pub unsafe fn read_status(&self) -> u8 {
        (self.sync.read().bits() as u8) & 0xF
    }
    pub unsafe fn enable_recv_irq(&self) {
        self.control
            .modify(|i| i | IPCCNT::ENABLE_RECV_FIFO_IRQ | IPCCNT::ENABLE_SEND_FIFO_IRQ);
    }
    pub unsafe fn send_value_raw(&self, value: u32) -> Result<(), SendFifoError> {
        let control = self.control.read();
        if control.contains(IPCCNT::SEND_FIFO_FULL) {
            return Err(SendFifoError::QueueFull);
        }
        if !control.contains(IPCCNT::ENABLE_FIFOS) {
            return Err(SendFifoError::FifoDisabled);
        }
        self.send.write(value);
        Ok(())
    }
    pub unsafe fn recieve_value_raw(&self) -> Result<u32, RecieveFifoError> {
        let control = self.control.read();
        if control.contains(IPCCNT::RECV_FIFO_EMPTY) {
            return Err(RecieveFifoError::QueueEmpty);
        }
        if !control.contains(IPCCNT::ENABLE_FIFOS) {
            return Err(RecieveFifoError::FifoDisabled);
        }
        Ok(IPC_FIFO_RECIEVE.read())
    }
    pub unsafe fn recieve_raw_blocking(&self) -> u32 {
        while self.recv_fifo_empty() {
            crate::swi_halt();
        }
        IPC_FIFO_RECIEVE.read()
    }
    pub unsafe fn send_raw_blocking(&self, value: u32) {
        while self.control.read().contains(IPCCNT::SEND_FIFO_FULL) {}
        self.send.write(value);
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct IPCSYNC: u32 {
        const REMOTE_CPU_STATUS = 0b1111;
        const HOST_CPU_STATUS = 0b1111 << 8;
        const SEND_IRQ = (1<<13);
        const ENABLE_IRQ = (1<<14);
    }
    #[derive(Debug, Clone, Copy)]
    pub struct IPCCNT: u32 {
        const SEND_FIFO_EMPTY = (1<<0);
        const SEND_FIFO_FULL = (1<<1);
        const ENABLE_SEND_FIFO_IRQ = (1<<2);
        const FLUSH_SEND_FIFO = (1<<3);

        const RECV_FIFO_EMPTY = (1<<8);
        const RECV_FIFO_FULL = (1<<9);
        const ENABLE_RECV_FIFO_IRQ = (1<<10);

        const ERROR = (1<<14);
        const ENABLE_FIFOS = (1<<15);
    }
}

pub struct IPCSendHandle;
impl IPCRecvHandle {
    pub unsafe fn recieve(&self) -> Result<u32, RecieveFifoError> {
        IPC_FIFO_HARDWARE.recieve_value_raw()
    }
}
pub struct IPCRecvHandle;
impl IPCSendHandle {
    pub unsafe fn send(&self, value: u32) -> Result<(), SendFifoError> {
        IPC_FIFO_HARDWARE.send_value_raw(value)
    }
}
pub unsafe trait IPCSend {
    fn send(self);
    fn recieve() -> Self;
}
unsafe impl IPCSend for alloc::string::String {
    fn send(self) {
        let s = IPCSendHandle;
        let (a, b, c) = self.into_raw_parts();
        unsafe {
            s.send(a as u32).unwrap();
            s.send(b as u32).unwrap();
            s.send(c as u32).unwrap();
        }
    }

    fn recieve() -> Self {
        let s = IPCRecvHandle;
        unsafe {
            let a = s.recieve().unwrap();
            let b = s.recieve().unwrap();
            let c = s.recieve().unwrap();
            Self::from_raw_parts(a as *mut u8, b as usize, c as usize)
        }
    }
}
unsafe impl<T> IPCSend for alloc::vec::Vec<T, Global> {
    fn send(self) {
        let s = IPCSendHandle;
        let (a, b, c) = self.into_raw_parts();
        unsafe {
            s.send(a as u32).unwrap();
            s.send(b as u32).unwrap();
            s.send(c as u32).unwrap();
        }
    }

    fn recieve() -> Self {
        let s = IPCRecvHandle;
        unsafe {
            let a = s.recieve().unwrap();
            let b = s.recieve().unwrap();
            let c = s.recieve().unwrap();
            Self::from_raw_parts(a as *mut T, b as usize, c as usize)
        }
    }
}
/*
macro_rules! ipc_types {
    ($($variant:ident($inner:ty)),* $(,)? ) => {
        pub enum IPCcomms {
            $(
                $variant($inner)
            ),*
        }
        $(
            impl Into<IPCcomms> for $inner  {
                fn into(self) -> IPCcomms {
                    IPCcomms::$variant(self)
                }
            }
        ),*
        impl IPCcomms {
            pub fn send(item: impl Into<IPCcomms>) {
                let item = item.into();
                let sender = IPCSendHandle;
                match item {
                    $(
                        Self::$variant(inner) => inner.send(sender),
                    )*
                }
            }
            pub fn recieve(item: impl Into<IPCcomms>) {
                let item = item.into();
                let sender = IPCSendHandle;
                match item {
                    $(
                        Self::$variant(inner) => inner.send(sender),
                    )*
                }
            }
        }

    };
}
ipc_types!(String(alloc::string::String));
*/
