//
//
extern crate tar;
extern crate nix;

use std::error::Error;

use std::path::{Path,PathBuf};
use std::fs::{DirBuilder,File};

use nix::unistd::{chdir,chroot,execve};
use std::ffi::CString;

use self::tar::Archive;

fn image_path(image_name: &str, image_dir: &str) -> PathBuf {
    Path::new(image_dir).join(image_name).with_extension("tar")
}

fn container_path(container_id: &str, container_dir: &str) -> PathBuf {
    Path::new(container_dir).join(container_id)
}

fn create_container_root(image_name: &str, image_dir: &str,
                         container_id: &str, container_dir: &str
                        ) -> PathBuf {
    let image_path = image_path(image_name, image_dir);
    let container_root = container_path(container_id, container_dir);

    if  !image_path.exists() {
        panic!("OS Image doesn't exist: {:?}", image_path);
    }

    if !container_root.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&container_root).unwrap();
    }

    let tar = File::open(image_path).unwrap();
    let mut archive = Archive::new(tar);
    archive.unpack(&container_root).unwrap();

    container_root
}

pub fn contain(command: &CString, args: &[CString],
               image_name: &str, image_dir: &str,
               container_id: &str, container_dir: &str) {

    let container_root = create_container_root(image_name, image_dir,
                                               container_id, container_dir);

    // TODO: Create mounts


    chroot(&container_root).unwrap_or_else(|e| panic!("mount failed: {}", e));

    chdir("/").unwrap_or_else(|e| panic!("Could not chdir /: {}", e));

    let _process = match execve(command, args, &[]) {
        Err(why) => panic!("couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}