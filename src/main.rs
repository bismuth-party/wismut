extern crate clap;
extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate toml;

use clap::{Arg, App, SubCommand};
use futures::Stream;
use std::fs::File;
use std::io::prelude::*;
use tokio_core::reactor::Core;
use telegram_bot::*;
use toml::Value;

fn main() {
    let matches = App::new("Wismut")
       .version("0.1.0")
       .about("Telegram group statistics bot")
       .author("Bismuth")
       .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
       .get_matches();

    let conf_path = matches.value_of("config").unwrap_or("config.toml");
    let conf = load_config();
    println!("{}", &conf["token"]);

    let mut core = Core::new().unwrap();

    let api = Api::configure(&conf["token"].as_str().unwrap()).build(core.handle()).unwrap();

    // Fetch new updates via long poll method
    let future = api.stream().for_each(|update| {

        // If the received update contains a new message...
        if let UpdateKind::Message(message) = update.kind {

            if let MessageKind::Text {ref data, ..} = message.kind {
                // Print received text message to stdout.
                println!("<{}>: {}", &message.from.first_name, data);

                // Answer message with "Hi".
                api.spawn(message.text_reply(
                    format!("Hi, {}! You just wrote '{}'", &message.from.first_name, data)
                ));
            }
        }

        Ok(())
    });

    core.run(future).unwrap();
}

fn load_config() -> Value {
    let mut f = File::open("config.toml").expect("No config found!");

    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("something went wrong reading the file");

    let value = contents.as_str().parse::<Value>().unwrap();

    return value;
}
