use ssh2::{Sftp, OpenFlags, OpenType};

fn main() {
    let sftp: Sftp = unsafe { std::mem::zeroed() };
    let _ = sftp.open_mode(
        std::path::Path::new("test"),
        OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
        0o644,
        OpenType::File
    );
}
