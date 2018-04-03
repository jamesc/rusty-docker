#![feature(libc)]

extern crate clap;
extern crate nix;
extern crate libc;
extern crate uuid;

use clap::{App, Arg, SubCommand};
use libc::_exit;
use nix::sys::wait::{waitpid,WaitStatus};
use nix::unistd::{fork,ForkResult};
use std::ffi::CString;
use uuid::{Uuid,UuidVersion};

mod isolate;

fn main() {

   let matches = App::new("rd")
        .about("Rusty Docker")
        .version("1.0")
        .author("Me")
        .subcommand(SubCommand::with_name("run")
            .about("starts a container")
            .arg(Arg::with_name("image-dir")
                 .help("Images directory")
                 .takes_value(true)
                 .long("image-dir")
                 .default_value("./images"))
            .arg(Arg::with_name("image-name")
                 .help("Image name")
                 .takes_value(true)
                 .short("i")
                 .long("image-name")
                 .default_value("ubuntu"))
            .arg(Arg::with_name("container-dir")
                 .help("Containers directory")
                 .takes_value(true)
                 .long("container-dir")
                 .default_value("./containers"))
            .arg(Arg::with_name("command")
                .help("COMMAND")
                .takes_value(true)
                .multiple(true)
                .required(true)))
        .get_matches();

    match matches.subcommand() {
        ("run", Some(run_matches)) => {
            let container_id = Uuid::new(UuidVersion::Random).unwrap();
            let values = run_matches.values_of("command").unwrap();
            let args = values.collect::<Vec<_>>();
            let command = args[0].clone();
            run(command, args,
                run_matches.value_of("image-name").unwrap(),
                run_matches.value_of("image-dir").unwrap(),
                &container_id.simple().to_string(),
                run_matches.value_of("container-dir").unwrap());
            },
        ("", None)   => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _            => unreachable!(), // If all subcommands are defined above, anything else is unreachable!()
    }
}

fn run(command: &str, args: Vec<&str>,
        image_name: &str, image_dir: &str,
        container_id: &str, container_dir: &str) {
    println!("DEBUG: args {:?}", args);
    // Allocate here so we only do async-safe work after the fork
    let c_command = CString::new(command).unwrap();
    let c_args = args.iter().map(|a| CString::new(*a).unwrap()).collect::<Vec<_>>();
    match fork().expect("fork failed") {
        ForkResult::Child => {
            isolate::contain(&c_command, c_args.as_slice(),
                             image_name, image_dir,
                             container_id, container_dir);
        }
        ForkResult::Parent{ child } => {
            // This is the parent, pid contains the PID of the forked process
            // wait for the forked child, fetch the exit status
            match waitpid(child, None) {
                Ok(WaitStatus::Exited(pid, 0)) if pid == child => {
                    println!("{} exited with status {}.", pid, 0)
                },
                _ => unsafe { _exit(1) },
            }
        }
    }
}