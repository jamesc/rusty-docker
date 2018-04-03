//
//
use std::process::{Command, Stdio};
use std::error::Error;

fn create_container_root(image_dir: &str,
                        image_name: &str,
                        container_dir: &str,
                        container_name: &str) -> String {
    println!("Image Name: {}/{}", image_dir, image_name);
    println!("Container Root: {}/{}", container_dir, container_name);

    return String::from(container_dir);
}

pub fn contain(command: &str, args: Vec<&str>,
                image_dir: &str,
                image_name: &str,
                container_dir: &str,
                container_name: &str) {

    let _container_root = create_container_root(image_dir, image_name,
                                                container_dir, container_name);

    let _process = match Command::new(command)
                                .args(args)
                                .stdin(Stdio::piped())
                                .spawn() {
        Err(why) => panic!("couldn't spawn process: {}", why.description()),
        Ok(process) => process,
    };
}