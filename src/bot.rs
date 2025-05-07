use crate::{
    application::{self, Application},
    watch_url_provider::WatchURLProvider,
};
use std::{fmt::Display, sync::Arc};
use teloxide::{
    dispatching::UpdateHandler,
    payloads::SendMessage,
    prelude::*,
    requests::JsonRequest,
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
    application: Arc<application::Application>,
    watch_url_provider: Arc<dyn WatchURLProvider + Send + Sync>,
) -> Dispatcher<Bot, Error, teloxide::dispatching::DefaultKey> {
    let bot = Bot::new(bot_token);

    bot.set_chat_menu_button()
        .menu_button(teloxide::types::MenuButton::Commands)
        .send()
        .await
        .expect("не удалось установить тип меню бота");
    bot.set_my_commands(Command::bot_commands())
        .send()
        .await
        .expect("не удалось установить список команд для бота");

    Dispatcher::builder(bot, build_handler())
        .dependencies(dptree::deps![application, watch_url_provider])
        .default_handler(default_handler)
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

async fn default_handler(upd: Arc<Update>) {
    let upd_as_json = match serde_json::to_string(upd.as_ref()) {
        Ok(json) => json,
        Err(err) => {
            log::error!("error while converting Update to json: {}", err);
            return;
        }
    };

    log::warn!("Unhandled update: {}", upd_as_json);
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

async fn next_episode_handler(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
    watch_url_provider: Arc<dyn WatchURLProvider + Send + Sync>,
) -> HandlerResult {
    send_next_episode_message(bot, msg, application, watch_url_provider)?.await?;

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> HandlerResult {
    let Some(data) = q.data.as_ref() else {
        log::error!("получили пустое поле data в колбеке");
        return Ok(());
    };

    log::info!("in callback_handler: data={data}");

    bot.answer_callback_query(&q.id).text("✅").await?;

    let Some(message) = q.regular_message() else {
        return Ok(());
    };
    let Some(text) = message.text() else {
        return Ok(());
    };

    bot.edit_text(message, format!("{text}\n\n✅ Просмотрено"))
        .await?;

    Ok(())
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
    watch_url_provider: Arc<dyn WatchURLProvider + Send + Sync>,
) -> HandlerResult {
    let text = match msg.text() {
        Some(text) => text,
        None => {
            send_help_message(bot, msg).await?;
            return Ok(());
        }
    };

    if text == MainKeyboardButtons::Moar.to_string() {
        send_next_episode_message(bot, msg, application, watch_url_provider)?.await?;
    } else if text == MainKeyboardButtons::ListSeenEpisodes.to_string() {
        log::info!("processing list seen episodes");
    } else {
        send_help_message(bot, msg).await?;
    };

    Ok(())
}

fn send_help_message(bot: Bot, msg: Message) -> JsonRequest<SendMessage> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_markup(build_main_keyboard())
}

fn send_next_episode_message(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
    watch_url_provider: Arc<dyn WatchURLProvider + Send + Sync>,
) -> Result<JsonRequest<SendMessage>, application::error::Error> {
    let user = msg.from.expect("should not be None at this point");
    let next_episode = application.get_next_episode(application::UserID::new(user.id.0))?;

    let watch_url = watch_url_provider.build_url(&next_episode);

    let response = format!(
        r#"
Предлагаю посмотреть:

Сезон {} серия {}

{watch_url}
"#,
        next_episode.season(),
        next_episode.episode(),
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Посмотрел",
        format!("mark_seen={}", next_episode.code()),
    )]]);

    Ok(bot
        .send_message(msg.chat.id, response.trim())
        .reply_markup(keyboard))
}
