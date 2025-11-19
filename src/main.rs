use clap::{Arg, Command, Subcommand};
use env_logger;
use log::info;

fn main() {
    env_logger::init();

    let matches = Command::new("hash-folderoo")
        .version("0.1.0")
        .about("Hash-based folder toolkit (prototype)")
        .subcommand_required(false)
        .subcommand(
            Command::new("hashmap").about("Create a hashmap of files in a directory").arg(
                Arg::new("path").short('p').long("path").takes_value(true).required(true),
            ),
        )
        .get_matches();

    if let Some(sub) = matches.subcommand_matches("hashmap") {
        let path = sub.get_one::<String>("path").expect("path");
        info!("Would compute hashmap for {}", path);
    } else {
        println!("Run with --help for usage");
    }
}
