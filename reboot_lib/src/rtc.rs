use volatile_register::RW;

use crate::MemoryWrapper;
pub struct RTCHardware(RW<RTCReg>);
impl RTCHardware {
    pub unsafe fn transact(&self, command: &[u8], result: &mut [u8]) {
        self.0.write(RTCReg::CS_0 | RTCReg::SCK_1 | RTCReg::SIO_1);
        crate::swi_delay(84);
        self.0.write(RTCReg::CS_1 | RTCReg::SCK_1 | RTCReg::SIO_1);
        crate::swi_delay(84);

        for mut byte in command.iter().copied() {
            for _bit in 0..8 {
                self.0.write(
                    RTCReg::CS_1
                        | RTCReg::SCK_0
                        | RTCReg::DATA_SELECT
                        | RTCReg::from_bits_retain(byte >> 7),
                );
                crate::swi_delay(84);
                self.0.write(
                    RTCReg::CS_1
                        | RTCReg::SCK_1
                        | RTCReg::DATA_SELECT
                        | RTCReg::from_bits_retain(byte >> 7),
                );
                crate::swi_delay(84);
                byte <<= 1;
            }
        }

        for byte in result {
            for bit in 0..8 {
                self.0.write(RTCReg::CS_1 | RTCReg::SCK_0);
                crate::swi_delay(84);
                self.0.write(RTCReg::CS_1 | RTCReg::SCK_1);
                crate::swi_delay(84);
                if self.0.read().contains(RTCReg::DATA_OUT) {
                    *byte |= (1 << bit);
                }
            }
        }
        self.0.write(RTCReg::CS_0 | RTCReg::SCK_1);
        crate::swi_delay(84);
    }
}
const RTC_HARDWARE: MemoryWrapper<RTCHardware> = MemoryWrapper(0x04000138 as *mut RTCHardware);
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct RTCReg: u8 {
        const DATA_OUT = (1<<0);
        const CLOCK_OUT = (1<<1);
        const SELECT_OUT = (1<<2);

        const DATA_SELECT = (1<<4);
        const CLOCK_SELECT = (1<<5);
        const SELECT_SELECT = (1<<6);

        const CS_0 = (1<<6);
        const CS_1 = (1<<6) | (1<<2);

        const SCK_0 = (1<<5);
        const SCK_1 = (1<<5) | (1<<1);

        const SIO_0 = (1<<4);
        const SIO_1 = (1<<4) | (1<<0);

    }
}
