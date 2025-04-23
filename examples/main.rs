use eframe::{
    App, NativeOptions,
    egui::{self, CentralPanel},
};
//use egui_video::{CpalAudioDevice, Player};
use media_player::{self, MediaPlayer, MediaType};
use std::fs::{self, File};

struct MyApp {
    media_player: MediaPlayer,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            media_player: MediaPlayer {
                media_type: MediaType::Audio,
            },
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Example");
        });
    }
}

fn main() {
    let _ = eframe::run_native(
        "Example",
        NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default()))),
    );
}
