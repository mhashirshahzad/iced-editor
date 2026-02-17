#![allow(unused_imports)]
#![allow(dead_code)]

use iced::Command;
use iced::Font;
use iced::Length;
use iced::executor;
use iced::futures::io;
use iced::highlighter::{self, Highlighter};
use iced::keyboard;
use iced::widget::button;
use iced::widget::{column, container, horizontal_space, pick_list, row, text, text_editor};
use iced::{Application, Element, Settings, Theme};

use std::path::{Path, PathBuf};
use std::sync::Arc;

fn main() -> iced::Result {
    Editor::run(Settings {
        default_font: Font::MONOSPACE,
        ..Settings::default()
    })
}

struct Editor {
    content: text_editor::Content,
    path: Option<PathBuf>,
    error: Option<Error>,
    theme: highlighter::Theme,
}
#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    New,
    Save,
    Open,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    FileSaved(Result<PathBuf, Error>),
    ThemeSelected(highlighter::Theme),
}

impl Application for Editor {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                path: None,
                content: text_editor::Content::new(),
                error: None,
                theme: highlighter::Theme::SolarizedDark,
            },
            Command::perform(load_file(default_file()), Message::FileOpened),
        )
    }

    fn title(&self) -> String {
        String::from("Bongo Editor")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::New => {
                self.path = None;
                self.content = text_editor::Content::new();
                Command::none()
            }
            Message::Edit(action) => {
                self.content.edit(action);
                self.error = None;
                Command::none()
            }
            Message::Save => {
                let text = self.content.text();

                Command::perform(save_file(self.path.clone(), text), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                Command::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
            Message::FileOpened(Ok((path, content))) => {
                self.path = Some(path);
                self.content = text_editor::Content::with(&content);
                Command::none()
            }
            Message::FileOpened(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
            Message::ThemeSelected(theme) => {
                self.theme = theme;
                Command::none()
            }
            Message::Open => Command::perform(pick_file(), Message::FileOpened),
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        keyboard::on_key_press(|key_code, modifiers| match key_code {
            keyboard::KeyCode::S if modifiers.command() => Some(Message::Save),
            _ => None,
        })
    }
    fn view(&self) -> iced::Element<'_, Self::Message> {
        let controls = row![
            button("New").on_press(Message::New),
            button("Open").on_press(Message::Open),
            button("Save").on_press(Message::Save),
            horizontal_space(Length::Fill),
            pick_list(
                highlighter::Theme::ALL,
                Some(self.theme),
                Message::ThemeSelected
            )
        ]
        .spacing(10);
        let input = text_editor(&self.content)
            .on_edit(Message::Edit)
            .highlight::<Highlighter>(
                highlighter::Settings {
                    theme: self.theme,
                    extension: self
                        .path
                        .as_ref()
                        .and_then(|path| path.extension()?.to_str())
                        .unwrap_or("rs")
                        .to_string(),
                },
                |highlight, _theme| highlight.to_format(),
            );

        let status_bar = {
            let status = if let Some(Error::IOFailed(error)) = self.error.as_ref() {
                text(error.to_string()).size(14)
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("- New File -"),
                }
            };
            let position = {
                let (line, col) = self.content.cursor_position();

                text(format!("{}:{}", line + 1, col + 1))
            };
            row![status, horizontal_space(Length::Fill), position]
        };

        container(column![controls, input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

#[derive(Debug, Clone)]
enum Error {
    DialogClosed,
    IOFailed(io::ErrorKind),
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), Error> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Choose a text file...")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;
    load_file(handle.path().to_owned()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), Error> {
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| error.kind())
        .map_err(Error::IOFailed)?;

    Ok((path, contents))
}

async fn save_file(path: Option<PathBuf>, text: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .set_title("Choose a file name...")
            .save_file()
            .await
            .ok_or(Error::DialogClosed)
            .map(|handle| handle.path().to_owned())?
    };

    tokio::fs::write(&path, text)
        .await
        .map_err(|error| Error::IOFailed(error.kind()))?;

    Ok(path)
}
fn default_file() -> PathBuf {
    PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")))
}
