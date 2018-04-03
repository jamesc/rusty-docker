#![feature(libc)]

extern crate clap;
extern crate nix;
extern crate libc;

use clap::{App, Arg, SubCommand};
use libc::_exit;
use nix::sys::wait::{waitpid,WaitStatus};
use nix::unistd::{fork,ForkResult};
use std::process::{Command, Stdio};
use std::error::Error;


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
            let mut values = run_matches.values_of("command").unwrap();
            let command = values.next().unwrap();
            let args = values.collect::<Vec<_>>();
            run(command, args);
            },
        ("", None)   => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _            => unreachable!(), // If all subcommands are defined above, anything else is unreachable!()
    }
}

fn contain(command: &str, args: Vec<&str>) {
    let _process = match Command::new(command)
                                .args(args)
                                .stdin(Stdio::piped())
                                .spawn() {
        Err(why) => panic!("couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}

fn run(command: &str, args: Vec<&str>) {
    match fork().expect("fork failed") {
        ForkResult::Child => {
            contain(command, args);
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