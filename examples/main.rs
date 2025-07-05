use eframe::{
    App, NativeOptions,
    egui::{self, CentralPanel, ComboBox, Sense, TextEdit},
};
use egui_player::{self, MediaType, Player, TranscriptionSettings};
use tokio::runtime::Runtime;

struct MyApp {
    player: Player,
    path: String,
    transcription_setting: TranscriptionSettings,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            player: Player::new("assets/Dreamweaver.mp3"),
            path: "assets/Dreamweaver.mp3".to_string(),
            transcription_setting: TranscriptionSettings::TranscriptLabel,
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("click to set path: ");
                let tedit_resp = ui.add_sized(
                    [ui.available_width(), ui.available_height()],
                    TextEdit::singleline(&mut self.path).interactive(false),
                );

                if ui
                    .interact(
                        tedit_resp.rect,
                        tedit_resp.id.with("click_sense"),
                        Sense::click(),
                    )
                    .clicked()
                {
                    if let Some(path_buf) = rfd::FileDialog::new()
                        .add_filter("audio", &["mp3", "wav", "m4a", "flac"])
                        .pick_file()
                    {
                        self.path = path_buf.as_path().to_string_lossy().to_string();
                        self.player = Player::new(&self.path);
                    }
                }
            });

            ui.separator();

            match self.player.media_type {
                MediaType::Audio => {
                    ui.heading("Audio");
                    ui.label("Please pause before switching files!");

                    ComboBox::from_label("Transcription options")
                        .selected_text(format!("{:?}", self.transcription_setting))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.transcription_setting,
                                TranscriptionSettings::None,
                                "No Transcription",
                            );
                            ui.selectable_value(
                                &mut self.transcription_setting,
                                TranscriptionSettings::Allow,
                                "Transcription Enabled",
                            );
                            ui.selectable_value(
                                &mut self.transcription_setting,
                                TranscriptionSettings::TranscriptLabel,
                                "Transcription Enabled with Label",
                            );
                            ui.selectable_value(
                                &mut self.transcription_setting,
                                TranscriptionSettings::ShowTimeStamps,
                                "Transcription with Timestamps",
                            );
                        });
                    self.player
                        .set_transcript_settings(self.transcription_setting);
                    self.player.ui(ui);
                }
                MediaType::Video => {
                    ui.heading("Video");
                    ui.label("Currently not supported, will be soon!");
                    ui.label("Please pause before switching files!");
                }
                MediaType::Error => {
                    ui.heading("Error");
                }
            }
        });
    }
}

fn main() {
    let rt = Runtime::new().unwrap();
    let _ = rt.block_on(async {
        eframe::run_native(
            "Player Example",
            NativeOptions::default(),
            Box::new(|_| Ok(Box::new(MyApp::default()))),
        )
    });
}
