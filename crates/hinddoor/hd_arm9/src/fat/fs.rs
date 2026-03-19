use crate::fat::{
    bs::{BadFSError, BootSector, FSType},
    SectorManager,
};

pub struct FileSystem<'a, T> {
    sector_manager: T,
    fs_type: FSType,
    buffer: &'a mut [reboot_lib::StorageSector],
}
pub enum FSMountError<T> {
    //Sector manager failed performing an operation
    IOError(T),
    //Filesystem is malformed (either isn't a FAT FS at all or a non-standard one)
    BadFS(BadFSError),
    //provided buffer is too small
    BufTooSmall,

    //Logical anomalies, deffered to reduce code size
    //If you get this error, please report it!
    LogicError,
}
impl<'a, T: SectorManager> FileSystem<'a, T> {
    pub fn new(
        mut sector_manager: T,
        buffer: &'a mut [reboot_lib::StorageSector],
    ) -> Result<FileSystem<T>, FSMountError<T::Error>> {
        // split off boot sector
        let Some((bs, _remainder)) = buffer.split_at_mut_checked(1) else {
            return Err(FSMountError::BufTooSmall);
        };

        // read boot sector
        sector_manager
            .read_sectors(0, bs)
            .map_err(|e| FSMountError::IOError(e))?;

        // load boot sector address safely
        // (should be infallibe, but good luck telling the compiler which would still bloat this with panic symbols as of 2025-12-29)
        let Some(bs_sector) = bs.get_mut(0) else {
            return Err(FSMountError::LogicError);
        };

        // evaluate the boot sector to find fs type, it should be valid if this works
        let fs_type = {
            //Guarantee safety in upcoming code with static assertions.
            const _: () = assert!(
                core::mem::size_of::<reboot_lib::StorageSector>()
                    == core::mem::size_of::<BootSector>()
            );
            const _: () = assert!(
                core::mem::align_of::<reboot_lib::StorageSector>()
                    >= core::mem::align_of::<BootSector>()
            );

            //Safety: The size and alignment of StorageSector as well as the layout of BootSector
            //guarantees that a StorageSector is also a valid BootSector.
            let bs: &mut BootSector = unsafe { core::mem::transmute(bs_sector) };
            //dw bro i thought about it
            #[cfg(target_endian = "big")]
            bs.flip_endians();

            bs.evaluate().map_err(|e| FSMountError::BadFS(e))?
        };

        // We're all good!
        Ok(Self {
            sector_manager,
            fs_type,
            buffer,
        })
    }
}
