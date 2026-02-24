use age::secrecy::{ExposeSecret, SecretString};
use anyhow::Result;
use arcstr::ArcStr;
use embed_it::Embed;
use iced::advanced::svg::Handle;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, svg, text, text_editor,
    text_input, toggler,
};
use iced::{Element, Fill, Length, Task, Theme};
use paper_age::{convenience::create_pdf, page::PageSize};
use rfd::FileHandle;
use std::io::Cursor;
use std::sync::Arc;

#[derive(Embed)]
#[embed(path = "$CARGO_MANIFEST_DIR/assets", support_alt_separator)]
pub struct Assets;

fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init().expect("Initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }

    #[cfg(not(target_arch = "wasm32"))]
    tracing_subscriber::fmt::init();

    iced::application(App::default, App::update, App::view)
        .theme(Theme::CatppuccinMocha)
        .centered()
        .run()
}

type ArcBytes = Arc<[u8]>;

struct App {
    title: ArcStr,
    passphrase: SecretString,
    secret_content: text_editor::Content,
    secret_file_name: Option<ArcStr>,
    secret_file_content: Option<ArcBytes>,
    secret_file_loading: bool,
    is_file_secret: bool,
    notes_label: ArcStr,
    show_extra: bool,
    secret_warning: Option<ArcStr>,
    passphrase_warning: Option<ArcStr>,
    generate_warning: Option<ArcStr>,
    is_generating: bool,
    page_size: PageSize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: Default::default(),
            passphrase: Default::default(),
            secret_content: Default::default(),
            secret_file_name: Default::default(),
            secret_file_content: Default::default(),
            secret_file_loading: Default::default(),
            is_file_secret: Default::default(),
            notes_label: Default::default(),
            show_extra: Default::default(),
            secret_warning: Default::default(),
            passphrase_warning: Default::default(),
            generate_warning: Default::default(),
            is_generating: Default::default(),
            page_size: PageSize::A4,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    PassphraseChanged(String),
    SecretContentChanged(text_editor::Action),
    SecretFileChanged(ArcBytes),
    SecretFileLoad(Option<FileHandle>),
    SecretFilePick,
    PageSizeChanged(PageSize),
    NotesLabelChanged(String),
    ToggleExtraSpoiler,
    GeneratePdf,
    SaveSecretPdf(ArcBytes),
    GenerateDone,
    SecretWarning(ArcStr),
    GenerateWarning(ArcStr),
    PassphraseWarning(ArcStr),
    ToggleSecretSource(bool),
    ResetWarning,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for Message {}

impl App {
    fn update(&mut self, event: Message) -> Task<Message> {
        match event {
            Message::TitleChanged(data) => {
                self.title = data.into();
                Task::none()
            }
            Message::PassphraseChanged(data) => {
                self.passphrase = data.into();
                Task::none()
            }
            Message::SecretContentChanged(action) => {
                self.secret_content.perform(action);
                Task::none()
            }
            Message::NotesLabelChanged(data) => {
                self.notes_label = data.into();
                Task::none()
            }
            Message::ToggleExtraSpoiler => {
                self.show_extra = !self.show_extra;
                Task::none()
            }
            Message::GeneratePdf => {
                if self.is_generating {
                    return Task::none();
                }
                self.is_generating = true;
                Task::done(Message::ResetWarning).chain(
                    Task::future(App::generate_pdf(
                        self.title.clone(),
                        self.notes_label.clone(),
                        self.page_size.clone(),
                        if self.is_file_secret {
                            self.secret_file_content.clone()
                        } else {
                            Some(self.secret_content.text().trim().as_bytes().into())
                        },
                        self.passphrase.clone(),
                    ))
                    .then(|v| Task::batch(v.into_iter().map(Task::done)))
                    .chain(Task::done(Message::GenerateDone)),
                )
            }
            Message::SaveSecretPdf(content) => {
                Task::perform(Self::save_pdf(content), |x| x).then(|_| Task::none())
            }
            Message::GenerateDone => {
                self.is_generating = false;
                Task::none()
            }
            Message::SecretWarning(warning) => {
                self.secret_warning = Some(warning);
                Task::none()
            }
            Message::PassphraseWarning(warning) => {
                self.passphrase_warning = Some(warning);
                Task::none()
            }
            Message::ResetWarning => {
                self.passphrase_warning = None;
                self.secret_warning = None;
                self.generate_warning = None;
                Task::none()
            }
            Message::GenerateWarning(warning) => {
                self.generate_warning = Some(warning);
                Task::none()
            }
            Message::ToggleSecretSource(b) => {
                self.is_file_secret = b;
                Task::none()
            }
            Message::SecretFileChanged(content) => {
                self.secret_file_content = Some(content);
                Task::none()
            }
            Message::SecretFilePick => {
                if self.secret_file_loading {
                    Task::none()
                } else {
                    Task::perform(App::pick_secret(), Message::SecretFileLoad)
                }
            }
            Message::SecretFileLoad(handle) => {
                if let Some(f) = handle {
                    self.secret_file_name = Some(f.file_name().into());
                    Task::perform(
                        async move { f.read().await.into() },
                        Message::SecretFileChanged,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PageSizeChanged(page_size) => {
                self.page_size = page_size;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let logo = svg(Handle::from_memory(Assets.logo().content()))
            .height(Length::Fixed(100.0))
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text),
            });
        let extra_arrow_icon = if self.show_extra {
            svg(Handle::from_memory(
                Assets.icons().arrow_drop_down_line().content(),
            ))
        } else {
            svg(Handle::from_memory(
                Assets.icons().arrow_drop_right_line().content(),
            ))
        }
        .height(Length::Fixed(24.0))
        .width(Length::Fixed(12.0))
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().background),
        })
        .content_fit(iced::ContentFit::ScaleDown);
        let extra_button =
            button(row![extra_arrow_icon, "Extra"].align_y(iced::alignment::Vertical::Center))
                .on_press(Message::ToggleExtraSpoiler);
        let extra_config = if self.show_extra {
            column![
                extra_button,
                text("Title:"),
                text_input("PaperAge", &self.title).on_input(Message::TitleChanged),
                text("Notes Label:"),
                text_input("Notes Label", &self.notes_label).on_input(Message::NotesLabelChanged),
                text("Page Size:"),
                pick_list(
                    [PageSize::A4, PageSize::Letter,],
                    Some(self.page_size.clone()),
                    Message::PageSizeChanged,
                ),
            ]
        } else {
            column![extra_button,]
        };
        let secret_input = if self.is_file_secret {
            column![
                row![
                    button("Open").on_press(Message::SecretFilePick).style(
                        if self.secret_file_loading {
                            button::secondary
                        } else {
                            button::primary
                        }
                    ),
                    container(
                        text(
                            self.secret_file_name
                                .as_ref()
                                .map(ArcStr::as_str)
                                .unwrap_or_default()
                        )
                        .width(Length::Fill)
                    )
                    .padding(15),
                ]
                .align_y(iced::alignment::Vertical::Center),
                text(
                    self.secret_warning
                        .as_ref()
                        .map(ArcStr::as_str)
                        .unwrap_or_default()
                )
                .size(10)
                .style(text::danger),
            ]
        } else {
            column![
                text_editor(&self.secret_content).on_action(Message::SecretContentChanged),
                text(
                    self.secret_warning
                        .as_ref()
                        .map(ArcStr::as_str)
                        .unwrap_or_default()
                )
                .size(10)
                .style(text::danger),
            ]
        };
        scrollable(
            container(
                container(
                    column![
                        logo,
                        container(text("Paper Age").size(35)).center_x(Fill),
                        row![
                            text("Secret:"),
                            horizontal_space(),
                            toggler(self.is_file_secret)
                                .label("File")
                                .on_toggle(Message::ToggleSecretSource),
                        ],
                        secret_input,
                        text("Passphrase:"),
                        text_input("Passphrase", self.passphrase.expose_secret())
                            .on_input(Message::PassphraseChanged)
                            .secure(true),
                        text(
                            self.passphrase_warning
                                .as_ref()
                                .map(ArcStr::as_str)
                                .unwrap_or_default()
                        )
                        .size(10)
                        .style(text::danger),
                        extra_config,
                        container(
                            column![
                                button("Generate PDF").on_press(Message::GeneratePdf).style(
                                    if self.is_generating {
                                        button::secondary
                                    } else {
                                        button::primary
                                    }
                                ),
                                text(
                                    self.generate_warning
                                        .as_ref()
                                        .map(ArcStr::as_str)
                                        .unwrap_or_default()
                                )
                                .size(10)
                                .style(text::danger),
                            ]
                            .align_x(iced::alignment::Horizontal::Center)
                        )
                        .center_x(Fill),
                    ]
                    .spacing(10),
                )
                .max_width(400),
            )
            .padding(30)
            .center_x(Fill),
        )
        .into()
    }

    async fn generate_pdf(
        title: ArcStr,
        notes_label: ArcStr,
        page_size: PageSize,
        secret: Option<ArcBytes>,
        passphrase: SecretString,
    ) -> Vec<Message> {
        let secret_res = match secret {
            Some(secret_bytes) => {
                if secret_bytes.is_empty() {
                    Err("Secret is empty")
                } else {
                    Ok(secret_bytes)
                }
            }
            None => Err("Select file"),
        }
        .map_err(ArcStr::from)
        .map_err(Message::SecretWarning);
        let passphrase_res = if passphrase.expose_secret().is_empty() {
            Err("Passphrase is empty")
        } else {
            Ok(passphrase.clone())
        }
        .map_err(ArcStr::from)
        .map_err(Message::PassphraseWarning);
        let (secret, passphrase) = match (secret_res, passphrase_res) {
            (Ok(secret), Ok(passphrase)) => (secret, passphrase),
            (Err(e1), Ok(_)) => return vec![e1],
            (Ok(_), Err(e2)) => return vec![e2],
            (Err(e1), Err(e2)) => return vec![e1, e2],
        };
        let mut secret_reader = Cursor::new(secret);
        let pdf = match create_pdf(
            if title.is_empty() {
                "PaperAge".to_string()
            } else {
                title.to_string()
            },
            &mut secret_reader,
            passphrase.expose_secret(),
            Some(if notes_label.is_empty() {
                "Passphrase:".to_string()
            } else {
                notes_label.to_string()
            }),
            Some(false),
            Some(page_size),
            Some(false),
        ) {
            Ok(content) => content,
            Err(err) => return vec![Message::GenerateWarning(format!("Error: {}", err).into())],
        };
        vec![Message::SaveSecretPdf(pdf.into())]
    }

    async fn pick_secret() -> Option<FileHandle> {
        rfd::AsyncFileDialog::new().pick_file().await //.map(Mutex::new).map(Arc::new)
    }

    async fn save_pdf(content: ArcBytes) -> Result<()> {
        if let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("secret.pdf")
            .save_file()
            .await
        {
            file.write(&content).await?
        };
        Ok(())
    }
}

fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}
