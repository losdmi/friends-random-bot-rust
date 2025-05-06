use crate::application;
use std::{fmt::Display, sync::Arc};
use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    sugar::bot::BotMessagesExt,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup},
    utils::command::BotCommands,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
type HandlerResult = Result<(), Error>;

#[derive(Clone, Copy)]
enum MainKeyboardButtons {
    Moar,
    ListSeenEpisodes,
}

impl From<MainKeyboardButtons> for String {
    fn from(value: MainKeyboardButtons) -> Self {
        match value {
            MainKeyboardButtons::Moar => String::from("Ещё серию"),
            MainKeyboardButtons::ListSeenEpisodes => String::from("Просмотренные серии"),
        }
    }
}

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

pub async fn new(
    bot_token: String,
    application: Arc<application::Bot>,
) -> Dispatcher<Bot, Error, teloxide::dispatching::DefaultKey> {
    let bot = Bot::new(bot_token);

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
        .dependencies(dptree::deps![application])
        .default_handler(|upd| async move {
            log::warn!(
                "Unhandled update: {}",
                serde_json::to_string(upd.as_ref()).unwrap()
            );
        })
        .enable_ctrlc_handler()
        .build()
}

fn build_handler() -> UpdateHandler<Error> {
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

async fn help_handler(bot: Bot, msg: Message, application: Arc<application::Bot>) -> HandlerResult {
    log::info!("{}", application.0);

    send_help_message(bot, msg).await?;

    Ok(())
}

async fn next_episode_handler(bot: Bot, msg: Message) -> HandlerResult {
    send_next_episode_message(bot, msg).await?;

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> HandlerResult {
    let data = q.data.as_ref().expect("q.data must be not empty");

    log::info!("in callback_handler: data={data}");

    bot.answer_callback_query(&q.id).text("✅").await?;

    if let Some(message) = q.regular_message() {
        let text = message.text().unwrap().to_string();
        bot.edit_text(message, format!("{text}\n\n✅ Просмотрено"))
            .await?;
    }

    Ok(())
}

async fn message_handler(bot: Bot, msg: Message) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            send_help_message(bot, msg).await?;
            return Ok(());
        }
    };

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
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_markup(build_main_keyboard())
}

fn send_next_episode_message(
    bot: Bot,
    msg: Message,
) -> teloxide::requests::JsonRequest<teloxide::payloads::SendMessage> {
    let response = r#"
Предлагаю посмотреть:

Сезон 1 серия 3

url
"#;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Посмотрел",
        "mark_seen=s01e03",
    )]]);

    bot.send_message(msg.chat.id, response)
        .reply_markup(keyboard)
}
