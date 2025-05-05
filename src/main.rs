use config;
use log::LevelFilter;
use serde::Deserialize;
use std::fmt::Display;
use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    sugar::bot::BotMessagesExt,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup},
    utils::command::BotCommands,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy)]
enum MainKeyboardButtons {
    Moar,
    ListSeenEpisodes,
}

// impl Into<String> for MainKeyboardButtons {
//     fn into(self) -> String {
//         match self {
//             MainKeyboardButtons::Moar => String::from("Ещё серию"),
//             MainKeyboardButtons::ListSeenEpisodes => String::from("Просмотренные серии"),
//         }
//     }
// }

impl From<MainKeyboardButtons> for String {
    fn from(value: MainKeyboardButtons) -> Self {
        match value {
            MainKeyboardButtons::Moar => String::from("Ещё серию"),
            MainKeyboardButtons::ListSeenEpisodes => String::from("Просмотренные серии"),
        }
    }
}

// impl Into<&str> for MainKeyboardButtons {
//     fn into(self) -> &'static str {
//         match self {
//             MainKeyboardButtons::Moar => "Ещё серию",
//             MainKeyboardButtons::ListSeenEpisodes => "Просмотренные серии",
//         }
//     }
// }

impl Display for MainKeyboardButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(*self))
    }
}

/// Предлагаю для просмотра случайную серию сериала Друзья.
/// Когда хочется посмотреть Друзей, но лень выбирать конкретную серию.
///
/// Вот что я могу:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case")]
enum Command {
    #[command(hide)]
    Start,
    /// Показать текст помощи.
    Help,
    /// Предложить следующую серию.
    NextEpisode,
    /// Показать список просмотренных серий.
    ListSeenEpisodes,
}

#[derive(Debug, Deserialize)]
struct Config {
    bot_token: String,
}

impl Config {
    fn new() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::with_name("config.json").required(true))
            .build()?
            .try_deserialize()
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_env(env_logger::DEFAULT_FILTER_ENV)
        .init();

    log::info!("Reading config...");
    let config = match Config::new() {
        Ok(config) => config,
        Err(err) => {
            log::error!("{err}");
            return;
        }
    };

    log::info!("Starting bot...");

    let bot = Bot::new(config.bot_token);

    bot.set_chat_menu_button()
        .menu_button(teloxide::types::MenuButton::Commands)
        .send()
        .await
        .unwrap();
    bot.set_my_commands(Command::bot_commands())
        .send()
        .await
        .unwrap();

    Dispatcher::builder(bot, build_handler())
        .default_handler(|upd| async move {
            log::warn!(
                "Unhandled update: {}",
                serde_json::to_string(upd.as_ref()).unwrap()
            );
        })
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn build_handler() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .branch(case![Command::Start].endpoint(start_handler))
                .branch(case![Command::Help].endpoint(help_handler))
                .branch(case!(Command::NextEpisode).endpoint(next_episode_handler)),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_message().endpoint(message_handler))
}

fn build_main_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new(vec![
        vec![KeyboardButton::new(MainKeyboardButtons::Moar)],
        vec![KeyboardButton::new(MainKeyboardButtons::ListSeenEpisodes)],
    ])
    .resize_keyboard()
}

async fn start_handler(bot: Bot, msg: Message) -> HandlerResult {
    send_help_message(bot, msg).await?;

    Ok(())
}

async fn help_handler(bot: Bot, msg: Message) -> HandlerResult {
    send_help_message(bot, msg).await?;

    Ok(())
}

async fn next_episode_handler(bot: Bot, msg: Message) -> HandlerResult {
    send_next_episode_message(bot, msg).await?;

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> HandlerResult {
    let data = q
        .data
        .as_ref()
        .expect("q.data must be not empty");

    log::info!("in callback_handler: data={data}");

    bot.answer_callback_query(&q.id)
        .text("✅")
        .await?;

    if let Some(message) = q.regular_message() {
        let text = message
            .text()
            .unwrap()
            .to_string();
        bot.edit_text(message, format!("{text}\n\n✅ Просмотрено"))
            .await?;
    }

    Ok(())
}

async fn message_handler(bot: Bot, msg: Message) -> HandlerResult {
    let text = msg
        .text()
        .expect("message must not be empty");

    if text == MainKeyboardButtons::Moar.to_string() {
        send_next_episode_message(bot, msg).await?;
    } else if text == MainKeyboardButtons::ListSeenEpisodes.to_string() {
        log::info!("processing list seen episodes");
    } else {
        send_help_message(bot, msg).await?;
    };

    Ok(())
}

fn send_help_message(
    bot: Bot,
    msg: Message,
) -> teloxide::requests::JsonRequest<teloxide::payloads::SendMessage> {
    bot.send_message(
        msg.chat
            .id,
        Command::descriptions().to_string(),
    )
    .reply_markup(build_main_keyboard())
}

fn send_next_episode_message(
    bot: Bot,
    msg: Message,
) -> teloxide::requests::JsonRequest<teloxide::payloads::SendMessage> {
    let response = r#"
Предлагаю посмотреть:

Сезон 1, Серия 3

url
"#;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Посмотрел",
        "mark_seen=s01e03",
    )]]);

    bot.send_message(
        msg.chat
            .id,
        response,
    )
    .reply_markup(keyboard)
}
