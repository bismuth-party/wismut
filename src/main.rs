extern crate clap;
extern crate futures;
extern crate hyper;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate telegram_bot;
extern crate tokio_core;
extern crate toml;

use clap::{Arg, App};
use futures::Stream;
use serde_json::{Value as SValue, Error};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use tokio_core::reactor::Core;
use telegram_bot::*;
use toml::Value;


fn main() {
    // Clap CLI argument logic
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

    // Loading the config based on a given path or a fixed path
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

                if data.as_str() == "/info" {
                    api.spawn(message.text_reply(
                        format!("userid: {}\nchatid: {}", &message.from.id, &message.chat.id())
                    ));
                }


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

    // let mut map = HashMap::new();
    // map.insert("chatid", message.chat.id);
    // map.insert("userid", message.from.id);
    // map.insert("message", data);

    // let mut text = HashMap::new();
    // text.insert("text", "kaas");

    // let mut content = HashMap::new();
    // content.insert("type", 0);
    // content.insert("content", text);

    println!("{}", message.chat.id());

    let data = json!({
        "chatid": message.chat.id(),
        "userid": message.from.id,
        "message": {
            "type": 0,
            "content": {
                "text": data
            }
        }
    });

    let client = reqwest::Client::new();
    let res = client.post("http://api.bismuth.party/abcdef/message")
        .json(&data)
        .send()
        .unwrap();

    println!("{:?}", res);
}
