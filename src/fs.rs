use alloc::vec::Vec;
use uefi::{boot, fs::FileSystem, CString16};

pub fn read_file(path: &str) -> Vec<u8> {
    let path_as_cstr16 =
        CString16::try_from(path).expect("Path should only contain UCS2-compatible characters.");
    let sfs = boot::get_image_file_system(boot::image_handle()).unwrap();
    let mut fs = FileSystem::new(sfs);
    fs.read(path_as_cstr16.as_ref())
        .unwrap_or_else(|_| panic!("Should be able to read file \"{path}\""))
}
