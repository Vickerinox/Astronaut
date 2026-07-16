// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod driver;
pub trait SectorManager {
    type Error;
    fn read_sectors(
        &mut self,
        sector: u32,
        buffer: &mut [reboot_lib::StorageSector],
    ) -> Result<(), Self::Error>;
    fn write_sectors(
        &mut self,
        sector: u32,
        buffer: &[reboot_lib::StorageSector],
    ) -> Result<(), Self::Error>;
}
