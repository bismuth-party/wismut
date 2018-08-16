extern crate clap;
extern crate futures;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate telegram_bot;
extern crate tokio_core;
extern crate toml;
extern crate regex;
#[macro_use]
extern crate lazy_static;


use futures::Stream;
use telegram_bot::*;
use std::io::prelude::*;


lazy_static! {
    // NOTE: See https://regex101.com/r/P8zQTi/5
    static ref CMD_REGEX: regex::Regex = regex::Regex::new(r"(?i)^/([a-z_]*)(?: (.*))?$").unwrap();
}


static ROOT_URL: &'static str = "http://thorium.bismuth.party";


struct Config {
    pub bot_token: String,

    // NOTE: Should be merged with bot_token
    pub api_token: String,
}


fn main() {
    // Clap CLI argument logic
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .get_matches();

    // Load the config based on a given path or a fixed path
    let conf_path = matches.value_of("config").unwrap_or("config.toml");
    let conf = load_config_file(conf_path).unwrap();

    // Extract token from config
    let token = conf["token"].as_str().unwrap();

    // Prepare bot
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let api = Api::configure(token).build(core.handle()).unwrap();

    // Create global config struct to keep track of the token(s),
    // since we'll need it/them in backend API calls
    let config = Config {
        bot_token: token.to_string(),

        // NOTE: Should be merged with bot_token
        api_token: "abcdef".to_string(),
    };

    // Get and handle all updates (long-polling)
    let future = api.stream().for_each(|update| {
        handle_update(&config, &api, update);
        Ok(())
    });

    // Start bot
    core.run(future).unwrap();
}


fn load_config_file(config_path: &str) -> Result<toml::Value, toml::de::Error> {
    let mut file = std::fs::File::open(config_path)
        .expect("Invalid config path, does the config file exist?");

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Something went wrong reading the file");

    contents.as_str().parse()
}


/// Send a POST request to the specified URL with the specified data
/// The URL should *NOT* start with a /
fn post(url: &str, config: &Config, data: &serde_json::Value) -> serde_json::Value {
    let uri = &format!("{}/{}/{}", ROOT_URL, config.api_token, url);
    let res = reqwest::Client::new()
        .post(uri)
        .json(data)
        .send().unwrap()
        .text().unwrap();

    // Parse response as JSON
    let body = serde_json::from_str(&res).unwrap();
    println!("post/body = {:?}", body);

    body
}


/// Send a GET request to the specified URL
/// The URL should *NOT* start with a /
fn get(url: &str, config: &Config) -> serde_json::Value {
    let uri = &format!("{}/{}/{}", ROOT_URL, config.api_token, url);
    let res = reqwest::Client::new()
        .get(uri)
        .send().unwrap()
        .text().unwrap();

    // Parse response as JSON
    let body = serde_json::from_str(&res).unwrap();
    println!("get/body = {:?}", body);

    body
}


fn handle_update(config: &Config, api: &telegram_bot::Api, update: telegram_bot::Update) {
    if let UpdateKind::Message(message) = update.kind {

        match message.kind {
            MessageKind::Text { .. } => {
                handle_text(&config, &api, &message);
            },

            MessageKind::NewChatTitle { .. } => {
                handle_title(&config, &api, &message);
            },

            // TODO: Add remaining MessageKinds
            //       See https://github.com/telegram-rs/telegram-bot/blob/master/raw/src/types/message.rs#L83

            _ => {
                println!("\n\n\t\t!!!    unimplemented messagekind    !!!\n{:?}\n\n", message.kind);
            },
        }

    }
}


fn handle_text(config: &Config, api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Text { ref data, .. } = message.kind {
        // Log message
        println!("<{}>: {}", message.from.first_name, data);


        // Extract command and arguments
        if let Some(capt) = CMD_REGEX.captures(data) {
            let cmd  = capt.get(1).map_or("", |m| m.as_str());
            let args = capt.get(2).map_or("", |m| m.as_str());

            println!("cmd: {:?}\nargs: {:?}", &cmd, &args);
            handle_command(&config, &api, &message, &cmd, &args);
        }


        // Store message in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": {
                "id": message.from.id,
                "is_bot": false,
                "first_name": message.from.first_name,
                "last_name": message.from.last_name,
                "username": message.from.username,
                "language_code": "ding"
            },
            "message": {
                "type": 0,
                "content": {
                    "text": data,
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_command(config: &Config, api: &telegram_bot::Api, message: &telegram_bot::Message, cmd: &str, args: &str) {
    match cmd {
        "info" => {
            api.spawn(message.text_reply(format!(
                "userid: {}\nchatid: {}",
                message.from.id,
                message.chat.id(),
            )));
        },

        "token" => {
            let id = message.from.id;

            // Let backend generate token
            let body = get(&format!("generate_token/{}", id), &config);
            let token = body["token"].to_string();

            // Send to user in private
            let chat = ChatId::from(id);
            api.spawn(chat.text(format!("token: {}", token)));

            // Reply to message (in group?)
            // TODO: Remove for security purposes
            api.spawn(message.text_reply(format!("token: {}", token)));
        },

        "echo" => {
            api.spawn(message.text_reply(args).parse_mode(ParseMode::Markdown));
        },

        _ => {
            println!("Unknown command {:?}", cmd);
        },
    }
}


fn handle_title(config: &Config, api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::NewChatTitle { ref data, .. } = message.kind {
        // Store new title in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": {
                "id": message.from.id,
                "is_bot": false,
                "first_name": message.from.first_name,
                "last_name": message.from.last_name,
                "username": message.from.username,
                "language_code": "ding"
            },
            "title": data
        });

        post("chat_update/new_title", &config, &json);

    }
}
