extern crate clap;
extern crate futures;
extern crate hyper;
extern crate telegram_bot;
extern crate tokio_core;
extern crate toml;

use clap::{Arg, App};
use futures::Stream;
use hyper::{Method, Request};
use std::fs::File;
use std::io::prelude::*;
use tokio_core::reactor::Core;
use telegram_bot::*;
use toml::Value;


fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
       .version(env!("CARGO_PKG_VERSION"))
       .about(env!("CARGO_PKG_DESCRIPTION"))
       .author(env!("CARGO_PKG_AUTHORS"))

       .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))

       .get_matches();

    let conf_path = matches.value_of("config").unwrap_or("config.toml");
    let conf = load_config(conf_path).unwrap();

    let token = conf["token"].as_str().unwrap();
    println!("Token: {}", token);

    let mut core = Core::new().unwrap();

    let api = Api::configure(token)
        .build(core.handle())
        .unwrap();

    // Fetch new updates via long poll method
    let future = api.stream().for_each(|update| {

        // If the received update contains a new message...
        if let UpdateKind::Message(message) = update.kind {

            if let MessageKind::Text {ref data, ..} = message.kind {
                // Print received text message to stdout.
                handle_message(&message, data);


                // Answer message with "Hi".
                // api.spawn(message.text_reply(
                //     format!("Hi, {}! You just wrote '{}'", &message.from.first_name, data)
                // ));
            }
        }

        Ok(())
    });

    core.run(future).unwrap();
}


fn load_config(config_path: &str) -> Option<Value> {
    let mut f = File::open(config_path).expect("No config found!");

    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("Something went wrong reading the file");

    return contents.as_str().parse::<Value>().ok();
}


fn handle_message(message: &Message, data: &str) {
    println!("<{}>: {}", message.from.first_name, data);
}
