use av_codec::{common::CodecList, decoder::Decoder};
use eframe::{
    App, NativeOptions,
    egui::{self, CentralPanel, Vec2},
};
//use egui_video::{CpalAudioDevice, Player};
use av_format::{self, buffer::AccReader};
use media_player::{self, MediaPlayer};
use std::{
    fs::{self, File},
    io::BufReader,
    thread,
    time::Duration,
};

struct MyApp {
    media_player: MediaPlayer,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            media_player: MediaPlayer::new("assets\\beep.wav"),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Example");
            self.media_player.ui(ui);
        });
    }
}

fn main() {
    let _ = eframe::run_native(
        "Example",
        NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default()))),
    );
    // rodio_test();
    // av_test();
}

fn rodio_test() {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let file = File::open("assets/Dreamweaver.mp3").unwrap();
    let beep = stream_handle.play_once(BufReader::new(file)).unwrap();
    beep.set_volume(0.2);
    beep.try_seek(Duration::from_nanos(12e+9 as u64)).unwrap();

    println!("Started beep");
    loop {}
}

fn av_test() {
    let file = File::open("assets/Dreamweaver loop.mp3").unwrap();
    let acc_reader = AccReader::new(file);
}
