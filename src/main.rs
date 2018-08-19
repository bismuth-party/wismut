extern crate clap;
extern crate futures;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
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


struct Config {
    pub root_url: String,
    pub cobalt_root_url: String,

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
    let root_url = conf["root_url"].as_str().unwrap();
    let cobalt_root_url = conf["cobalt_root_url"].as_str().unwrap();

    // Prepare bot
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let api = Api::configure(token).build(core.handle()).unwrap();

    // Create global config struct to keep track of the token(s),
    // since we'll need it/them in backend API calls
    let config = Config {
        root_url: root_url.to_string(),
        cobalt_root_url: cobalt_root_url.to_string(),

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
    let file = std::fs::File::open(config_path);
    let mut file = file.expect("Invalid config path, does the config file exist?");

    let mut contents = String::new();
    file.read_to_string(&mut contents).expect(
        "Something went wrong reading the file",
    );

    contents.as_str().parse()
}


/// Send a POST request to the specified URL with the specified data
/// The URL should *NOT* start with a /
fn post(url: &str, config: &Config, data: &serde_json::Value) -> serde_json::Value {
    let uri = &format!("{}/{}/{}", config.root_url, config.api_token, url);
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
    let uri = &format!("{}/{}/{}", config.root_url, config.api_token, url);
    let res = reqwest::Client::new()
        .get(uri)
        .send().unwrap()
        .text().unwrap();

    // Parse response as JSON
    let body = serde_json::from_str(&res).unwrap();
    println!("get/body = {:?}", body);

    body
}


fn user_to_json(user: &telegram_bot::types::User) -> serde_json::Value {
    json!({
        "id": user.id,
        "is_bot": user.is_bot,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "username": user.username,
        "language_code": user.language_code,
    })
}


fn handle_update(config: &Config, api: &telegram_bot::Api, update: telegram_bot::Update) {
    if let UpdateKind::Message(message) = update.kind {
        match message.kind {
            MessageKind::Text { .. } => {
                handle_text(&config, &api, &message);
            }

            MessageKind::Audio { .. } => {
                handle_audio(&config, &api, &message);
            }

            MessageKind::Document { .. } => {
                handle_document(&config, &api, &message);
            }

            // TODO: animation

            MessageKind::Photo { .. } => {
                handle_photo(&config, &api, &message);
            }

            MessageKind::Sticker { .. } => {
                handle_sticker(&config, &api, &message);
            }

            MessageKind::Video { .. } => {
                handle_video(&config, &api, &message);
            }

            MessageKind::Voice { .. } => {
                handle_voice(&config, &api, &message);
            }

            MessageKind::VideoNote { .. } => {
                handle_video_note(&config, &api, &message);
            }

            MessageKind::Contact { .. } => {
                handle_contact(&config, &api, &message);
            }

            MessageKind::Location { .. } => {
                handle_location(&config, &api, &message);
            }

            MessageKind::Venue { .. } => {
                handle_venue(&config, &api, &message);
            }

            MessageKind::NewChatTitle { .. } => {
                handle_title(&config, &api, &message);
            }

            // TODO: Add remaining MessageKinds
            //       See https://github.com/telegram-rs/telegram-bot/blob/master/raw/src/types/message.rs#L83
            _ => {
                println!("\n\n\t\t!!!    unimplemented messagekind    !!!\n{:?}\n\n", message.kind);
            }
        }
    }
}


fn handle_text(config: &Config, api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Text { ref data, .. } = message.kind {
        // Log message
        println!("<{}>: {}", message.from.first_name, data);


        // Extract command and arguments
        if let Some(capt) = CMD_REGEX.captures(data) {
            let cmd = capt.get(1).map_or("", |m| m.as_str());
            let args = capt.get(2).map_or("", |m| m.as_str());

            println!("cmd: {:?}\nargs: {:?}", &cmd, &args);
            handle_command(&config, &api, &message, &cmd, &args);
        }

        // Store message in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
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


fn handle_command(
    config: &Config,
    api: &telegram_bot::Api,
    message: &telegram_bot::Message,
    cmd: &str,
    args: &str,
) {
    match cmd {
        "info" => {
            api.spawn(message.text_reply(format!(
                "userid: {}\nchatid: {}",
                message.from.id,
                message.chat.id(),
            )));
        }

        "token" => {
            api.spawn(
                message
                    .text_reply("Click [here](t.me/wismut_bot?start=HELLOGIMMETOKENPLS) to get your token")
                    .parse_mode(ParseMode::Markdown)
            );
        }

        "echo" => {
            api.spawn(message.text_reply(args).parse_mode(ParseMode::Markdown));
        }

        "start" if args == "HELLOGIMMETOKENPLS" => {
            let id = message.from.id;

            // Let backend generate token
            let body = get(&format!("generate_token/{}", id), &config);
            let token = body["token"].as_str().unwrap();

            // Send to user in private
            let chat = ChatId::from(id);
            api.spawn(
                chat
                    .text(format!("Click [here]({}/#{}) to go to the dashboard", config.cobalt_root_url, &token))
                    .parse_mode(ParseMode::Markdown)
            );
        }

        _ => {
            println!("Unknown command {:?}", cmd);
        }
    }
}


fn handle_audio(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Audio { ref data, .. } = message.kind {
        // Store audio in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 1,
                "content": {
                    "caption": "",
                    "file_id": data.file_id,
                    "duration": data.duration,
                    "performer": data.performer,
                    "title": data.title,
                    "mime_type": data.mime_type,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_document(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Document { ref data, ref caption, .. } = message.kind {
        // Store a document in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 2,
                "content": {
                    "caption": caption,
                    "file_id": data.file_id,
                    "file_name": data.file_name,
                    "mime_type": data.mime_type,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_photo(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Photo { ref data, ref caption, .. } = message.kind {
        let mut p_data_json = Vec::new();
        for photo in data {
            let json = json!({
                "file_id": photo.file_id,
                "width": photo.width,
                "height": photo.height,
                "file_size": photo.file_size,
            });

            p_data_json.push(json);
        }


        #[derive(Serialize)]
        struct Content {
            #[serde(skip_serializing_if = "Option::is_none")]
            caption: Option<String>,
            photo: serde_json::Value,
        }

        #[derive(Serialize)]
        struct Message {
            #[serde(rename = "type")]
            _type: isize,
            content: Content,
        }

        #[derive(Serialize)]
        struct Body {
            chatid: telegram_bot::ChatId,
            user: serde_json::Value,
            message: Message,
        }


        let body = Body {
            chatid: message.chat.id(),
            user: user_to_json(&message.from),
            message: Message {
                _type: 5,
                content: Content {
                    caption: caption.clone(),
                    photo: p_data_json.get(0).unwrap().clone(),
                },
            },
        };

        post("message", &config, &json!(body));
    }
}



fn handle_sticker(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Sticker { ref data, .. } = message.kind {
        // Store sticker in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 6,
                "content": {
                    "file_id": data.file_id,
                    "emoji": data.emoji,
                    "set_name": data.set_name,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_video(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Video { ref data, ref caption, .. } = message.kind {
        // Store audio in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 7,
                "content": {
                    "caption": caption,
                    "file_id": data.file_id,
                    "duration": data.duration,
                    "mime_type": data.mime_type,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_voice(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Voice { ref data, .. } = message.kind {
        // Store audio in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 8,
                "content": {
                    "caption": "",
                    "file_id": data.file_id,
                    "duration": data.duration,
                    "mime_type": data.mime_type,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_video_note(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::VideoNote { ref data, .. } = message.kind {
        // Store video note in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 9,
                "content": {
                    "file_id": data.file_id,
                    "duration": data.duration,
                    "file_size": data.file_size
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_contact(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Contact { ref data, .. } = message.kind {
        // Store contact in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 10,
                "content": {
                    "phone_number": data.phone_number,
                    "first_name": data.first_name,
                    "last_name": data.last_name,
                    "user_id": data.user_id,
                    "vcard": ""
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_location(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Location { ref data, .. } = message.kind {
        // Store location in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 11,
                "content": {
                    "longitude": data.longitude,
                    "latitude": data.latitude
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_venue(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::Venue { ref data, .. } = message.kind {
        // Store venue in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "message": {
                "type": 12,
                "content": {
                    "location": {
                        "longitude": data.location.longitude,
                        "latitude": data.location.latitude
                    },
                    "title": data.title,
                    "address": data.address
                },
            },
        });

        post("message", &config, &json);
    }
}


fn handle_title(config: &Config, _api: &telegram_bot::Api, message: &Message) {
    if let MessageKind::NewChatTitle { ref data, .. } = message.kind {
        // Store new title in backend
        let json = json!({
            "chatid": message.chat.id(),
            "user": user_to_json(&message.from),
            "title": data
        });

        post("chat_update/new_title", &config, &json);
    }
}
