use eframe::{
    App, NativeOptions,
    egui::{self, CentralPanel},
};
use media_player::{self, MediaPlayer};
use tokio::runtime::Runtime;

struct MyApp {
    media_player: MediaPlayer,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            media_player: MediaPlayer::new("assets/voice_test.mp3"),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Audio");
            self.media_player.ui(ui);
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
