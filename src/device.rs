#![allow(non_camel_case_types)]
extern crate lazy_static;

use lazy_static::initialize;
use nix::sys::stat::*;
use std::os::unix::fs::symlink;
use nix::mount::*;
use std::path::{Path};

use std::fs::{DirBuilder};
use nix::sys::stat::mknod;
//use nix::sys::stat::{fchmodat,FchmodatFlags};
use nix::unistd::{chown};
use nix::sys::stat::{Mode, S_IFCHR};

pub struct LinuxDevice {
    pub path: String,
    pub major: u64,
    pub minor: u64,
}

lazy_static! {
    static ref DEFAULT_DEVICES: Vec<LinuxDevice> = {
        let mut v = Vec::new();
        v.push(LinuxDevice{
            path: "null".to_string(),
            major: 1,
            minor: 3,
        });
        v.push(LinuxDevice{
            path: "zero".to_string(),
            major: 1,
            minor: 5,
        });
        v.push(LinuxDevice{
            path: "full".to_string(),
            major: 1,
            minor: 7,
        });
        v.push(LinuxDevice{
            path: "tty".to_string(),
            major: 5,
            minor: 0,
        });
        v.push(LinuxDevice{
            path: "urandom".to_string(),
            major: 1,
            minor: 9,
        });
        v.push(LinuxDevice{
            path: "random".to_string(),
            major: 1,
            minor: 8,
        });
        v
    };
}

pub fn make_devices(dev_root: &Path) {
    const NONE: Option<&'static [u8]> = None;
    // initialize static variables before forking
    initialize(&DEFAULT_DEVICES);
    // initialize(&NAMESPACES);

    let old = umask(Mode::from_bits_truncate(0o000));
    // Make some devices
    let devpts_path = dev_root.join("pts");
    if !devpts_path.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&devpts_path).unwrap();
        mount(Some("devpts"), &devpts_path, Some("devpts"),
              MS_NOSUID | MS_NOEXEC | MS_NOATIME, NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    }

    let devices = ["stdin", "stdout", "stderr"];
    for (i, dev) in devices.iter().enumerate() {
        symlink(Path::new("/proc/self/fd").join(i.to_string()),
                dev_root.join(dev)).unwrap_or_else(|e| panic!("ERROR: Symlink failed: {}", e));;
    }

    for dev in DEFAULT_DEVICES.iter() {
        println!("DEBUG: mknoding {}", &dev.path);
        let path = dev_root.join(&dev.path);
        mknod(&path.to_str(), S_IFCHR, Mode::from_bits_truncate(0o666),
              makedev(dev.major, dev.minor)).unwrap();
        //fchmodat(None, &path, Mode::all(), FollowSymlink);

        chown(&path.to_str(), Some(0), Some(0)).unwrap();
    }

    umask(old);
}