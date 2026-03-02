#![feature(string_remove_matches)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs::File, panic::PanicHookInfo};
use tracing::level_filters::LevelFilter;

use crate::ui::App;

mod ui;
mod message;
mod messangers;
mod message_server;
mod send_categories;
#[cfg(test)]
mod test;

fn panic_message_box(info: &PanicHookInfo) {
    rfd::MessageDialog::new()
    .set_buttons(rfd::MessageButtons::Ok)
    .set_title("Виявлена критична помилка")
    .set_level(rfd::MessageLevel::Error)
    .set_description(info.to_string())
    .show();
}

fn main() {
    std::panic::set_hook(Box::new(panic_message_box));

    #[cfg(debug_assertions)]
    {
        tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_max_level(LevelFilter::INFO)
        .with_writer(std::io::stdout)
        .init();
    }

    #[cfg(not(debug_assertions))]
    {
        let log_file = File::create("sender.log").unwrap();
        tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_max_level(LevelFilter::WARN)
        .with_writer(log_file)
        .with_ansi(false)
        .init();
    }

    iced::application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view,
    )
    .title("Modern Sender v1.0-rc3")
    .theme(App::theme)
    .font(include_bytes!("Roboto-VariableFont_wdth,wght.ttf"))
    .font(include_bytes!("ui/icons/MaterialIcons-Regular.ttf"))
    .default_font(iced::Font::with_name("Roboto"))
    .exit_on_close_request(false)
    .run()
    .unwrap()
}
