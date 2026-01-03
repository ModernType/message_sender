use derive_more::Display;
use iced::theme::{Base, Mode, palette::Extended};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Display, PartialEq)]
#[serde(into = "String", from = "&str")]
pub enum Theme {
    #[display("System")]
    None,
    System,
    #[display("System")]
    Dark,
    #[display("System")]
    Light,
    #[display("{_0}")]
    Selected(iced::Theme),
}

impl From<&str> for Theme {
    fn from(value: &str) -> Self {
        match value {
            "Light" => Self::Selected(iced::Theme::Light),
            "Dark" => Self::Selected(iced::Theme::Dark),
            "Dracula" => Self::Selected(iced::Theme::Dracula),
            "Nord" => Self::Selected(iced::Theme::Nord),
            "Solarized Light" => Self::Selected(iced::Theme::SolarizedLight),
            "Solarized Dark" => Self::Selected(iced::Theme::SolarizedDark),
            "Gruvbox Light" => Self::Selected(iced::Theme::GruvboxLight),
            "Gruvbox Dark" => Self::Selected(iced::Theme::GruvboxDark),
            "Catppuccin Latte" => Self::Selected(iced::Theme::CatppuccinLatte),
            "Catppuccin Frappé" => Self::Selected(iced::Theme::CatppuccinFrappe),
            "Catppuccin Macchiato" => Self::Selected(iced::Theme::CatppuccinMacchiato),
            "Catppuccin Mocha" => Self::Selected(iced::Theme::CatppuccinMocha),
            "Tokyo Night" => Self::Selected(iced::Theme::TokyoNight),
            "Tokyo Night Storm" => Self::Selected(iced::Theme::TokyoNightStorm),
            "Tokyo Night Light" => Self::Selected(iced::Theme::TokyoNightLight),
            "Kanagawa Wave" => Self::Selected(iced::Theme::KanagawaWave),
            "Kanagawa Dragon" => Self::Selected(iced::Theme::KanagawaDragon),
            "Kanagawa Lotus" => Self::Selected(iced::Theme::KanagawaLotus),
            "Moonfly" => Self::Selected(iced::Theme::Moonfly),
            "Nightfly" => Self::Selected(iced::Theme::Nightfly),
            "Oxocarbon" => Self::Selected(iced::Theme::Oxocarbon),
            "Ferra" => Self::Selected(iced::Theme::Ferra),
            _ => Self::System
        }
    }
}

impl From<Theme> for String {
    fn from(value: Theme) -> Self {
        value.to_string()
    }
}

impl From<iced::Theme> for Theme {
    fn from(value: iced::Theme) -> Self {
        match value {
            iced::Theme::CatppuccinLatte => Self::Light,
            iced::Theme::Dracula => Self::Dark,
            iced::Theme::Custom(_) => Self::None,
            theme => Self::Selected(theme),
        }
    }
}

impl From<Mode> for Theme {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Light => Self::Light,
            Mode::Dark => Self::Dark,
            Mode::None => Self::Light,
        }
    }
}

impl Theme {
    const LIGHT_THEME: &iced::Theme = &iced::Theme::CatppuccinLatte;
    const DARK_THEME: &iced::Theme = &iced::Theme::Dracula;
    pub const ALL: &'static [Self] = &[
        Self::System,
        Self::Selected(iced::Theme::Light),
        Self::Selected(iced::Theme::Dark),
        Self::Selected(iced::Theme::Dracula),
        Self::Selected(iced::Theme::Nord),
        Self::Selected(iced::Theme::SolarizedLight),
        Self::Selected(iced::Theme::SolarizedDark),
        Self::Selected(iced::Theme::GruvboxLight),
        Self::Selected(iced::Theme::GruvboxDark),
        Self::Selected(iced::Theme::CatppuccinLatte),
        Self::Selected(iced::Theme::CatppuccinFrappe),
        Self::Selected(iced::Theme::CatppuccinMacchiato),
        Self::Selected(iced::Theme::CatppuccinMocha),
        Self::Selected(iced::Theme::TokyoNight),
        Self::Selected(iced::Theme::TokyoNightStorm),
        Self::Selected(iced::Theme::TokyoNightLight),
        Self::Selected(iced::Theme::KanagawaWave),
        Self::Selected(iced::Theme::KanagawaDragon),
        Self::Selected(iced::Theme::KanagawaLotus),
        Self::Selected(iced::Theme::Moonfly),
        Self::Selected(iced::Theme::Nightfly),
        Self::Selected(iced::Theme::Oxocarbon),
        Self::Selected(iced::Theme::Ferra),
    ];

    pub fn as_theme(&self) -> &iced::Theme {
        match self {
            Self::Light | Self::None | Self::System => Self::LIGHT_THEME,
            Self::Dark => Self::DARK_THEME,
            Self::Selected(theme) => theme,
        }
    }

    pub fn is_system(&self) -> bool {
        !matches!(self, Self::Selected(_))
    }

    pub fn extended_palette(&self) -> &Extended {
        self.as_theme().extended_palette()
    }
}

impl Base for Theme {
    fn base(&self) -> iced::theme::Style {
        self.as_theme().base()
    }

    fn default(preference: Mode) -> Self {
        match preference {
            Mode::Dark => Self::Dark,
            Mode::Light => Self::Light,
            Mode::None => Self::None,
        }
    }

    fn mode(&self) -> Mode {
        if let Self::None = self {
            Mode::None
        }
        else {
            self.as_theme().mode()
        }
    }

    fn name(&self) -> &str {
        if let Self::None = self {
            "System"
        }
        else {
            self.as_theme().name()
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        Some(self.as_theme().palette())
    }
}

impl iced::widget::svg::Catalog for Theme {
    type Class<'a> = iced::widget::svg::StyleFn<'a, Self>;
    
    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_theme, _status| iced::widget::svg::Style::default())
    }

    fn style(&self, class: &Self::Class<'_>, status: iced::widget::svg::Status) -> iced::widget::svg::Style {
        class(self, status)
    }
}