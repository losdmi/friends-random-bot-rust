mod callback;

use crate::{
    application::{self, Application, Episode},
    error, watch_url_provider,
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
type WatchURLProvider = dyn watch_url_provider::WatchURLProvider + Send + Sync;

#[derive(Clone, Copy)]
enum MainKeyboardButtons {
    Moar,
    #[allow(dead_code)]
    ListSeenEpisodes,
    #[allow(dead_code)]
    ClearSeenEpisodes,
}

impl From<MainKeyboardButtons> for String {
    fn from(value: MainKeyboardButtons) -> Self {
        match value {
            MainKeyboardButtons::Moar => String::from("Ещё серию"),
            MainKeyboardButtons::ListSeenEpisodes => String::from("Показать просмотренные серии"),
            MainKeyboardButtons::ClearSeenEpisodes => String::from("Очистить просмотренные серии"),
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
    /// Очистить список просмотренных серий.
    ClearSeenEpisodes,
}

pub async fn new(
    bot_token: String,
    application: Arc<application::Application>,
    watch_url_provider: Arc<WatchURLProvider>,
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
                .branch(case!(Command::NextEpisode).endpoint(next_episode_handler))
                .branch(case!(Command::ListSeenEpisodes).endpoint(list_seen_episodes_handler))
                .branch(case!(Command::ClearSeenEpisodes).endpoint(clear_seen_episodes_handler)),
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
        // vec![KeyboardButton::new(MainKeyboardButtons::ListSeenEpisodes)],
        // vec![KeyboardButton::new(MainKeyboardButtons::ClearSeenEpisodes)],
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
    watch_url_provider: Arc<WatchURLProvider>,
) -> HandlerResult {
    send_next_episode_message(bot, msg, application, watch_url_provider)?.await?;

    Ok(())
}

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    application: Arc<Application>,
) -> HandlerResult {
    bot.answer_callback_query(&q.id).text("✅").await?;

    let Some(data) = q.data.as_ref() else {
        log::error!("получили пустое поле data в колбеке");
        return Ok(());
    };

    let command = match callback::Command::from_data_string(data) {
        Ok(command) => command,
        Err(err) => {
            log::error!("ошибка при парсинге колбек-команды: {}", err);
            return Ok(());
        }
    };

    match command {
        callback::Command::MarkSeen(parameter) => {
            handle_callback_mark_seen(bot, q, application, &parameter).await?
        }
        callback::Command::ClearSeenEpisodes(option) => {
            handle_callback_clear_seen_episodes(bot, q, application, option).await?
        }
    }

    Ok(())
}

async fn handle_callback_mark_seen(
    bot: Bot,
    q: CallbackQuery,
    application: Arc<Application>,
    parameter: &str,
) -> HandlerResult {
    application.mark_seen(
        application::UserID::new(q.from.id.0),
        Episode::from(parameter),
    )?;

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

async fn handle_callback_clear_seen_episodes(
    bot: Bot,
    q: CallbackQuery,
    application: Arc<Application>,
    option: callback::ClearSeenEpisodesOption,
) -> HandlerResult {
    let Some(message) = q.regular_message() else {
        return Ok(());
    };
    let Some(text) = message.text() else {
        return Ok(());
    };

    match option {
        callback::ClearSeenEpisodesOption::No => {
            bot.edit_text(
                message,
                format!("{text}\n\n❌ Очистка списка просмотренных серий отменена."),
            )
            .await?;

            Ok(())
        }
        callback::ClearSeenEpisodesOption::Yes => {
            let user = q.from.clone();
            application.clear_seen_episodes(application::UserID::new(user.id.0))?;

            bot.edit_text(
                message,
                format!("{text}\n\n✅ Список просмотренных серий очищен."),
            )
            .await?;

            Ok(())
        }
    }
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
    watch_url_provider: Arc<WatchURLProvider>,
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
    // } else if text == MainKeyboardButtons::ListSeenEpisodes.to_string() {
    // send_seen_episodes(bot, msg, application)?.await?;
    // } else if text == MainKeyboardButtons::ClearSeenEpisodes.to_string() {
    // send_clear_seen_episodes_confirmation_request(bot, msg)?.await?;
    } else {
        send_help_message(bot, msg).await?;
    };

    Ok(())
}

async fn list_seen_episodes_handler(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
) -> HandlerResult {
    send_seen_episodes(bot, msg, application)?.await?;

    Ok(())
}

async fn clear_seen_episodes_handler(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
) -> HandlerResult {
    send_clear_seen_episodes_confirmation_request(bot, msg, application)?.await?;

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
    watch_url_provider: Arc<WatchURLProvider>,
) -> Result<JsonRequest<SendMessage>, application::Error> {
    let user = msg.from.expect("should not be None at this point");

    let next_episode = match application.get_next_episode(application::UserID::new(user.id.0)) {
        Ok(next_episode) => next_episode,
        Err(application::Error::NoUnseenEpisodes) => {
            return Ok(bot
                .send_message(msg.chat.id, "Не осталось непросмотренных серий 🙂")
                .reply_markup(build_main_keyboard()));
        }
        Err(other) => {
            log::error!("unexpected error: {}", other);
            return Err(other);
        }
    };

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

fn send_seen_episodes(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
) -> Result<JsonRequest<SendMessage>, application::Error> {
    let user = msg.from.expect("should not be None at this point");
    let seen_episodes = application.list_seen_episodes(application::UserID::new(user.id.0))?;

    if seen_episodes.is_empty() {
        let text = r#"
Вы ещё не посмотрели ни одной серии.

Воспользуйтесь командой /next_episode чтобы узнать свою следующую серию для просмотра.
"#;

        return Ok(bot
            .send_message(msg.chat.id, text.trim())
            .reply_markup(build_main_keyboard()));
    }

    fn episode_to_string(episode: &Episode) -> String {
        format!("Сезон {} серия {}", episode.season(), episode.episode())
    }

    let text = format!(
        r#"
Просмотренные серии. Наверху недавние, внизу старые:

{}
"#,
        seen_episodes.iter().fold(String::new(), |acc, ep| format!(
            "{}\n{}",
            episode_to_string(ep),
            acc,
        )),
    );

    Ok(bot
        .send_message(msg.chat.id, text.trim())
        .reply_markup(build_main_keyboard()))
}

fn send_clear_seen_episodes_confirmation_request(
    bot: Bot,
    msg: Message,
    application: Arc<Application>,
) -> Result<JsonRequest<SendMessage>, application::Error> {
    let user = msg.from.expect("should not be None at this point");
    let seen_episodes = application.list_seen_episodes(application::UserID::new(user.id.0))?;

    if seen_episodes.is_empty() {
        return Ok(bot
            .send_message(
                msg.chat.id,
                "Нечего очищать, список просмотренных серий пуст.",
            )
            .reply_markup(build_main_keyboard()));
    }

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Да", "clear_seen_episodes=yes"),
        InlineKeyboardButton::callback("Нет", "clear_seen_episodes=no"),
    ]]);

    Ok(bot
        .send_message(
            msg.chat.id,
            "Вы точно хотите очистить список просмотренных серий?",
        )
        .reply_markup(keyboard))
}
