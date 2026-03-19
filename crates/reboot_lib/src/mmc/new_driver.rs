pub enum Command<'a> {
    GoIdleState,
    SendOptionalCondation(OCR),
    AllSendCID,
    SendRelativeAddr(RCA),
    //CMD4: Reserved
    //CMD5: Reserved
    //CMD6: Reserved
    SelectCard(RCA),
    //CMD8: SD Only
    SendCSD(RCA),
    SendCID(RCA),

    //CMD11: ???
    StopTransmittion,
    SendStatus(RCA),
    //CMD14: reserved,
    GoInactive(RCA),

    SetBlockLen(u32),
    ReadSingle(&'a mut [crate::StorageSector], u32),
    ReadMultiple(&'a mut [crate::StorageSector], u32),
    //CMD19: Reserved
    WriteSingle(&'a [crate::StorageSector], u32),
    WriteMultiple(&'a [crate::StorageSector], u32),

    ProgramCSD,

    SetWriteProtection(u32),
    ClearWriteProection(u32),
    SendWriteProtection(u32),
    //CMD31: Reserved
}

pub enum Response {
    None,
}
pub struct RCA(u16);
bitflags::bitflags! {

    pub struct OCR: u32 {

    }
    /// # An R1 Type response
    ///
    /// ### From MMC documentation:
    ///
    /// The card sends this response token after every command with the exception of
    /// SEND_STATUS commands. It is one-byte long, the MSB is always set to zero, and the
    /// other bits are error indications (1= error).
    ///
    pub struct Response1: u8 {
        /// The card is in an idle state and running initializing process.
        const IS_IDLE = (1<<0);
        /// An erase sequence was cleared before execution because an out-oferase sequence command was received.
        const ERASE_RESET = (1<<1);
        /// An illegal command code was detected.
        const ILLEGAL_COMMAND = (1<<2);
        /// The CRC check of the last command failed.
        const ERROR_COMM_CRC = (1<<3);
        /// An error in the sequence of erase commands occurred.
        const ERROR_ERASE_SEQ = (1<<4);
        /// A misaligned address that did not match the block length was used in the command.
        const ERROR_ADDRESS = (1<<5);
        /// The command’s argument (e.g., address, block length) was out of the allowed range for this card.
        const ERROR_PARAMETER = (1<<6);
    }
    pub struct Response2: u16 {

    }
}
