//
//
extern crate tar;
extern crate nix;
extern crate lazy_static;

use std::error::Error;
use std::path::{Path,PathBuf};
use std::fs::{DirBuilder,File,remove_dir};
use nix::unistd::{chdir,execvp,pivot_root};
use std::ffi::CString;
use nix::mount::{mount,MsFlags,umount2,MntFlags};
use nix::sched::{unshare,CloneFlags};
use self::tar::{Archive,EntryType};

use device::make_devices;

const NONE: Option<&'static [u8]> = None;

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

fn make_dirs(path: &PathBuf) {
    println!("Creating : {:?}", path);
    if !path.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(path).unwrap();
    }
}

fn create_container_root(image_name: &str, image_dir: &str,
                         container_id: &str, container_dir: &str
                        ) -> PathBuf {
    let image_path = image_path(image_name, image_dir);
    let image_root = Path::new(&image_dir).join(&image_name).join("rootfs");

    if  !image_path.exists() {
        panic!("ERROR: OS Image doesn't exist: {:?}", image_path);
    }

    if !&image_root.exists() {
        make_dirs(&image_root);
        let tar = File::open(image_path).unwrap();
        let mut archive = Archive::new(tar);
        for (_i, entry) in archive.entries().unwrap().enumerate() {
            let mut file = entry.unwrap();
            // Tar archives might contain devices or other odd things
            match file.header().entry_type() {
                EntryType::Block => {},
                EntryType::Char => {},
                _ => {
                    file.unpack_in(&image_root).unwrap_or_else(|e| panic!("ERROR: Couldn't untar OS: {}", e));
                }
            }
        }
    }

    // Setup CoW FS
    let container_rootfs = container_path(container_id, container_dir, &["rootfs"]);
    make_dirs(&container_rootfs);
    let container_cow_rw = container_path(container_id, container_dir, &["cow_rw"]);
    make_dirs(&container_cow_rw);
    let container_cow_workdir = container_path(container_id, container_dir, &["cow_workdir"]);
    make_dirs(&container_cow_workdir);

    // Create a new mount point at the root for pivot_root
    let args = format!("lowerdir={},upperdir={},workdir={}",
                    image_root.to_str().unwrap(),
                    container_cow_rw.to_str().unwrap(),
                    container_cow_workdir.to_str().unwrap());
    mount(Some("overlay"), &container_rootfs, Some("overlay"),
          MsFlags::MS_NODEV, Some(args.as_str())).unwrap_or_else(|e| panic!("ERROR: Couldn't mount overlayfs: {}", e));

    container_rootfs
}

pub fn contain(command: &CString, args: &[CString],
               image_name: &str, image_dir: &str,
               container_id: &str, container_dir: &str) {
    unshare(CloneFlags::CLONE_NEWNS).unwrap_or_else(|e| panic!("ERROR: Couldn't unshare mount: {}", e));

    mount(Some("rootfs"), Path::new("/"), Some("lxfs"),
          MsFlags::MS_PRIVATE | MsFlags::MS_REC, NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));

    let container_root = create_container_root(image_name, image_dir,
                                               container_id, container_dir);
    println!("DEBUG: Created a new root fs for our container: {:?}", container_root);

    mount(Some("proc"), &container_root.join("proc"), Some("proc"),
          MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC, NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    mount(Some("sysfs"), &container_root.join("sys"), Some("sysfs"),
          MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC, NONE).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));
    mount(Some("udev"), &container_root.join("dev"), Some("tmpfs"),
          MsFlags::MS_NOATIME, Some("mode=755")).unwrap_or_else(|e| panic!("ERROR: Mount failed: {}", e));

    make_devices(&container_root.join("dev"));

    let old_root = &container_root.join("old_root");
    if !old_root.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&old_root).unwrap();
    }
    pivot_root(&container_root,
               &container_root.join("old_root")).unwrap_or_else(|e| panic!("Error: pivot_root failed: {}", e));

    chdir("/").unwrap_or_else(|e| panic!("ERROR: Could not chdir /: {}", e));

    umount2("/old_root", MntFlags::MNT_DETACH).unwrap();
    remove_dir("/old_root").unwrap();

    let _process = match execvp(command, args) {
        Err(why) => panic!("ERROR: Couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}