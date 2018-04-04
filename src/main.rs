#![feature(libc)]

extern crate clap;
extern crate nix;
extern crate libc;

use clap::{App, Arg, SubCommand};
use libc::_exit;
use nix::sys::wait::{waitpid,WaitStatus};
use nix::unistd::{fork,ForkResult,execve};
use std::error::Error;
use std::ffi::CString;

fn main() {

   let matches = App::new("rd")
        .about("Rusty Docker")
        .version("1.0")
        .author("Me")
        .subcommand(SubCommand::with_name("run")
            .about("starts a container")
            .arg(Arg::with_name("command")
                .help("COMMAND")
                .takes_value(true)
                .multiple(true)
                .required(true)))
        .get_matches();

    match matches.subcommand() {
        ("run", Some(run_matches)) => {
            let values = run_matches.values_of("command").unwrap();
            let args = values.collect::<Vec<_>>();
            let command = args[0].clone();
            run(command, args);
        },
        ("", None)   => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _            => unreachable!(), // If all subcommands are defined above, anything else is unreachable!()
    }
}

fn contain(command: &CString, args: &[CString]) {
    println!("DEBUG: execve: {:?} {:?}", command, args);
    let _process = match execve(command, args, &[]) {
        Err(why) => panic!("couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}

fn run(command: &str, args: Vec<&str>) {
    println!("DEBUG: args {:?}", args);
    // Allocate here so we only do async-safe work after the fork
    let c_command = CString::new(command).unwrap();
    let c_args = args.iter().map(|a| CString::new(*a).unwrap()).collect::<Vec<_>>();
    match fork().expect("fork failed") {
        ForkResult::Child => {
            contain(&c_command, c_args.as_slice());
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