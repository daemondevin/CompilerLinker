use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    os::windows::io::AsRawHandle,
    path::PathBuf,
    ptr,
};

use owo_colors::OwoColorize;
use structopt::StructOpt;

#[link(name = "kernel32")]
extern "system" {
    fn CreateDirectoryW(
        lpPathName: *const u16,
        lpSecurityAttributes: *const std::ffi::c_void,
    ) -> i32;

    fn CreateSymbolicLinkW(
        lpSymlinkFileName: *const u16,
        lpTargetFileName: *const u16,
        dwFlags: u32,
    ) -> u8;
}

const SYMBOLIC_LINK_FLAG_DIRECTORY: u32 = 1;
const SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE: u32 = 2;

#[derive(StructOpt, Debug)]
#[structopt(name = "CompilerLinker", about = "Portable symbolic, junction, and hard/soft link creator.")]
struct CliOpts {
    #[structopt(short = "t", long = "link", help = "Source path where link will be created")]
    src: PathBuf,

    #[structopt(short = "o", long = "target", help = "Destination path the link points to")]
    dst: PathBuf,

    #[structopt(short = "s", long = "soft", help = "Create a soft link (directory symlink)")]
    soft: bool,

    #[structopt(short = "h", long = "hard", help = "Create a hard link")]
    hard: bool,

    #[structopt(short = "d", long = "symbolic", help = "Create a symbolic link (file symlink)")]
    symbolic: bool,

    #[structopt(short = "j", long = "junction", help = "Create a junction point")]
    junction: bool,
}

#[derive(Debug, Clone, Copy)]
enum LinkType {
    Soft,
    Hard,
    Symbolic,
    Junction,
}

impl LinkType {
    fn name(self) -> &'static str {
        match self {
            LinkType::Soft => "Soft Link",
            LinkType::Hard => "Hard Link",
            LinkType::Symbolic => "Symbolic Link",
            LinkType::Junction => "Junction",
        }
    }
}

struct LinkError {
    message: String,
    exit_code: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv = CliOpts::from_args();

    let link_type = match parse_link_type(&argv) {
        Ok(lt) => lt,
        Err(e) => {
            eprintln!("{} {}", "error:".bright_red(), e.message.bright_red());
            std::process::exit(e.exit_code);
        }
    };

    if let Err(e) = create_link(link_type, &argv.src, &argv.dst) {
        eprintln!("{} {}", "error:".bright_red(), e.message.bright_red());
        std::process::exit(e.exit_code);
    }

    print_success(link_type, &argv.src, &argv.dst);
    Ok(())
}

fn parse_link_type(opts: &CliOpts) -> Result<LinkType, LinkError> {
    let count = [opts.soft, opts.hard, opts.symbolic, opts.junction]
        .iter()
        .filter(|&&b| b)
        .count();

    match count {
        0 => Err(LinkError {
            message: "No link type specified. Use --soft, --hard, --symbolic, or --junction.".to_string(),
            exit_code: 1,
        }),
        1 => {
            if opts.soft {
                Ok(LinkType::Soft)
            } else if opts.hard {
                Ok(LinkType::Hard)
            } else if opts.symbolic {
                Ok(LinkType::Symbolic)
            } else {
                Ok(LinkType::Junction)
            }
        }
        _ => Err(LinkError {
            message: "Multiple link types specified. Choose only one.".to_string(),
            exit_code: 1,
        }),
    }
}

fn create_link(link_type: LinkType, src: &PathBuf, dst: &PathBuf) -> Result<(), LinkError> {
    #[cfg(windows)]
    {
        use std::os::windows::fs as winfs;

        match link_type {
            LinkType::Junction => create_junction(dst, src),
            LinkType::Symbolic => winfs::symlink_file(dst, src).map_err(|e| LinkError {
                message: format!("Failed to create symbolic link: {}", e),
                exit_code: 3,
            }),
            LinkType::Hard => std::fs::hard_link(dst, src).map_err(|e| LinkError {
                message: format!("Failed to create hard link: {}", e),
                exit_code: 4,
            }),
            LinkType::Soft => winfs::symlink_dir(dst, src).map_err(|e| LinkError {
                message: format!("Failed to create soft link: {}", e),
                exit_code: 5,
            }),
        }
    }

    #[cfg(not(windows))]
    {
        Err(LinkError {
            message: "This utility only works on Windows.".to_string(),
            exit_code: 7,
        })
    }
}

fn utf16_encode(s: &std::path::Path) -> Vec<u16> {
    let mut encoded: Vec<u16> = OsStr::new(s.as_os_str())
        .encode_wide()
        .collect();
    encoded.push(0); // Null terminate
    encoded
}

fn create_junction(target: &PathBuf, link: &PathBuf) -> Result<(), LinkError> {
    unsafe {
        let link_wide = utf16_encode(link);
        let target_wide = utf16_encode(target);

        // Create the directory for the junction point
        if CreateDirectoryW(link_wide.as_ptr(), ptr::null()) == 0 {
            return Err(LinkError {
                message: "Failed to create directory for junction point".to_string(),
                exit_code: 2,
            });
        }

        // Create the junction using reparse points
        // A junction is created by writing a reparse point to the directory
        let reparse_data = create_reparse_data(&target_wide)?;

        let handle = std::fs::OpenOptions::new()
            .write(true)
            .open(link)
            .map_err(|e| LinkError {
                message: format!("Failed to open junction directory: {}", e),
                exit_code: 2,
            })?;

        #[allow(non_snake_case)]
        extern "system" {
            fn DeviceIoControl(
                hDevice: *mut std::ffi::c_void,
                dwIoControlCode: u32,
                lpInBuffer: *const std::ffi::c_void,
                nInBufferSize: u32,
                lpOutBuffer: *mut std::ffi::c_void,
                nOutBufferSize: u32,
                lpBytesReturned: *mut u32,
                lpOverlapped: *const std::ffi::c_void,
            ) -> i32;
        }

        const FSCTL_SET_REPARSE_POINT: u32 = 0x900A4;
        let mut bytes_returned = 0u32;

        if DeviceIoControl(
            handle.as_raw_handle() as *mut _,
            FSCTL_SET_REPARSE_POINT,
            reparse_data.as_ptr() as *const _,
            reparse_data.len() as u32,
            ptr::null_mut(),
            0,
            &mut bytes_returned,
            ptr::null(),
        ) == 0
        {
            return Err(LinkError {
                message: "Failed to set reparse point for junction".to_string(),
                exit_code: 2,
            });
        }

        Ok(())
    }
}

fn create_reparse_data(target: &[u16]) -> Result<Vec<u8>, LinkError> {
    const REPARSE_JUNCTION_DATA_BUFFER_HEADER_SIZE: usize = 8;
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA0000003;

    // Calculate sizes
    let target_len = (target.len() - 1) * 2; // -1 for null terminator
    let reparse_data_len = REPARSE_JUNCTION_DATA_BUFFER_HEADER_SIZE
        + target_len * 2
        + 4; // 4 bytes for null terminator space

    let mut buffer = vec![0u8; reparse_data_len + 8];

    // Write reparse tag
    let tag = IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes();
    buffer[0..4].copy_from_slice(&tag);

    // Write reparse data length
    let data_len = (reparse_data_len as u16).to_le_bytes();
    buffer[4..6].copy_from_slice(&data_len);

    // Write reserved field
    buffer[6..8].copy_from_slice(&[0u8; 2]);

    // Write path buffer offset and length
    let path_offset = 0u16.to_le_bytes();
    buffer[8..10].copy_from_slice(&path_offset);

    let path_len = (target_len as u16).to_le_bytes();
    buffer[10..12].copy_from_slice(&path_len);

    // Write the target path
    for (i, &wchar) in target.iter().take(target.len() - 1).enumerate() {
        let bytes = wchar.to_le_bytes();
        buffer[12 + i * 2..12 + i * 2 + 2].copy_from_slice(&bytes);
    }

    Ok(buffer)
}

fn print_success(link_type: LinkType, src: &PathBuf, dst: &PathBuf) {
    println!(
        "{} {} {} {:?}, {} {:?}",
        link_type.name().bright_green(),
        "created at source".bright_green(),
        "→".bright_green(),
        src.bright_green(),
        "pointing to".bright_green(),
        dst.bright_green()
    );
}
