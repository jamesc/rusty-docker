//
//
extern crate tar;
extern crate nix;

use std::error::Error;

use std::path::{Path,PathBuf};
use std::fs::{DirBuilder,File};
use std::os::unix::fs::symlink;

use nix::unistd::{chdir,chroot,execve};
use std::ffi::CString;

use nix::mount::{mount, MsFlags};

use self::tar::{Archive,EntryType};

fn image_path(image_name: &str, image_dir: &str) -> PathBuf {
    Path::new(image_dir).join(image_name).with_extension("tar")
}

fn container_path(container_id: &str, container_dir: &str, subdirs: &[&str]) -> PathBuf {
    let mut path = Path::new(container_dir).join(container_id);
    for d in subdirs {
        path.push(d)
    }
    path
}

fn create_container_root(image_name: &str, image_dir: &str,
                         container_id: &str, container_dir: &str
                        ) -> PathBuf {
    let image_path = image_path(image_name, image_dir);
    let container_root = container_path(container_id, container_dir, &["rootfs"]);

    if  !image_path.exists() {
        panic!("ERROR: OS Image doesn't exist: {:?}", image_path);
    }

    if !container_root.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&container_root).unwrap();
    }

    let tar = File::open(image_path).unwrap();
    let mut archive = Archive::new(tar);
    for (_i, entry) in archive.entries().unwrap().enumerate() {
        let mut file = entry.unwrap();
        // Tar archives might contain devices or other odd things
        match file.header().entry_type() {
            EntryType::Block => {},
            EntryType::Char => {},
            _ => {
                file.unpack_in(&container_root).unwrap_or_else(|e| panic!("ERROR: Couldn't untar OS: {}", e));
            }
        }
    }
    container_root
}

pub fn contain(command: &CString, args: &[CString],
               image_name: &str, image_dir: &str,
               container_id: &str, container_dir: &str) {
const NONE: Option<&'static [u8]> = None;
    let container_root = create_container_root(image_name, image_dir,
                                               container_id, container_dir);
    println!("DEBUG: Created a new root fs for our container: {:?}", container_root);

    // TODO: Create mounts with proper attributes
    mount(Some("proc"), &container_root.join("proc"), Some("proc"),
          MsFlags::empty(), NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    mount(Some("sysfs"), &container_root.join("sys"), Some("sysfs"),
          MsFlags::empty(), NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    mount(Some("none"), &container_root.join("dev"), Some("tmpfs"),
          MsFlags::empty(), NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));

    // Make some devices
    let devpts_path = container_root.join("dev").join("pts");
    if !devpts_path.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&devpts_path).unwrap();
        mount(Some("devpts"), &devpts_path, Some("devpts"),
            MsFlags::empty(), NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    }

    let devices = ["stdin", "stdout", "stderr"];
    for (i, dev) in devices.iter().enumerate() {
        symlink(Path::new("/proc/self/fd").join(i.to_string()),
                   container_root.join("dev").join(dev)).unwrap_or_else(|e| panic!("ERROR: Symlink failed: {}", e));;
    }

    chroot(&container_root).unwrap_or_else(|e| panic!("Error: Mount failed: {}", e));

    chdir("/").unwrap_or_else(|e| panic!("ERROR: Could not chdir /: {}", e));

    let _process = match execve(command, args, &[]) {
        Err(why) => panic!("ERROR: Couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}