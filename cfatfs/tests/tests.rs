mod simulated_driver;

use fatfs_embedded::fatfs::{self, File, FileOptions, FormatOptions};
use embassy_futures::block_on;

//Test function must be called "main" to satisfy ThreadModeRawMutex.
#[test]
fn main() {
    const TEST_STRING: &[u8] = b"Hello world!";
    //Create an instance of the simulated block storage device.
    let driver = simulated_driver::RamBlockStorage::new();
    //Install the driver.
    block_on(fatfs::diskio::install(driver));
    let mut locked_fs = block_on(fatfs::FS.lock());
    //Format the drive.
    locked_fs.mkfs("", FormatOptions::FAT32, 0, 0, 0, 0).expect("Formatting drive failed.");
    //Mount the drive.
    locked_fs.mount().expect("Mounting drive failed.");
    //Create a new test file.
    let mut test_file: File = locked_fs.open("test.txt", FileOptions::CreateAlways | FileOptions::Read | FileOptions::Write).expect("Opening failed.");
    //Write a test string to the file.
    locked_fs.write(&mut test_file, TEST_STRING).expect("Writing to the file failed.");
    //Seek back to the beginning of the file.
    locked_fs.seek(&mut test_file, 0).expect("Seeking to the beginning of the file failed.");
    //Read the string back from the file.
    let mut read_back: [u8; TEST_STRING.len()] = [0; TEST_STRING.len()];
    locked_fs.read(&mut test_file, &mut read_back).expect("Reading the file failed.");
    assert_eq!(TEST_STRING, read_back);
}