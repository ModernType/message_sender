#![feature(string_remove_matches)]

use std::fs::File;
use simplelog::Config;

use crate::ui::App;

mod ui;
mod message;
mod messangers;
mod message_server;
mod send_categories;
#[cfg(test)]
mod test;


fn main() {
    let log_file = File::create("sender.log").unwrap();
    #[cfg(debug_assertions)]
    let log_filter = log::LevelFilter::Info;
    #[cfg(not(debug_assertions))]
    let log_filter = log::LevelFilter::Warn;

    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            log_filter,
            Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        simplelog::WriteLogger::new(log_filter, Config::default(), log_file),
    ])
    .unwrap();

    iced::application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view,
    )
    .title("Modern Sender")
    .theme(App::theme)
    .font(include_bytes!("Roboto-VariableFont_wdth,wght.ttf"))
    .font(include_bytes!("MaterialIcons-Regular.ttf"))
    .default_font(iced::Font::with_name("Roboto"))
    .exit_on_close_request(false)
    .run()
    .unwrap()
}
