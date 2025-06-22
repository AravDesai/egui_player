use eframe::{
    App, NativeOptions,
    egui::{self, CentralPanel, Sense, TextEdit},
};
use media_player::{self, MediaPlayer};
use rfd;
use tokio::runtime::Runtime;

struct MyApp {
    media_player: MediaPlayer,
    media_path: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            media_player: MediaPlayer::new("assets/beep.wav"),
            media_path: "assets/beep.wav".to_string(),
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
                    TextEdit::singleline(&mut self.media_path).interactive(false),
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
                        .add_filter("audio", &["mp3", "ogg", "wav"])
                        .pick_file()
                    {
                        self.media_path = path_buf.as_path().to_string_lossy().to_string();
                        self.media_player = MediaPlayer::new(&self.media_path);
                    }
                }
            });

            match self.media_player.media_type {
                media_player::MediaType::Audio => {
                    ui.heading("Audio");
                    self.media_player.ui(ui);
                    ui.label("Audio Transcription:");
                    let media_player_transcript = match &self.media_player.transcript {
                        Some(transcript) => transcript,
                        None => "...",
                    };
                    ui.label(media_player_transcript);
                }
                media_player::MediaType::Video => {
                    ui.heading("Video");
                    ui.label("Currently not supported, will be soon!");
                }
                media_player::MediaType::Error => {
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
            "Media Player Example",
            NativeOptions::default(),
            Box::new(|_| Ok(Box::new(MyApp::default()))),
        )
    });
}
