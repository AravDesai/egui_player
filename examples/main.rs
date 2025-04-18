use ffmpeg_next as ffmpeg;
use ffmpeg_next::device::input;

use eframe::{self, NativeOptions};
use eframe::{App, egui};
use egui::CentralPanel;
use ffmpeg::format::input;
use rodio;
use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::Duration;

struct MyApp {
    variab: u64,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            variab: Default::default(),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {});
    }
}

fn main() {
    ffmpeg_audio_test();
    // let _ = eframe::run_native(
    //     "app",
    //     NativeOptions::default(),
    //     Box::new(|_| Ok(Box::new(MyApp::default()))),
    // );
}

fn ffmpeg_audio_test() {
    ffmpeg::init().unwrap();
    let unclean_beep = input("assets/beep.wav").unwrap();
    let beep = unclean_beep
        .streams()
        .best(ffmpeg::util::media::Type::Audio)
        .ok_or(ffmpeg::Error::StreamNotFound);

    // let context_decoder = ffmpeg::codec::context::Context::decoder(self);
    // let mut decoder = context_decoder.decoder().audio().unwrap();

    thread::sleep(Duration::from_millis(1500));
}

fn rodio_test() {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let file = File::open("assets/beep.wav").unwrap();
    let beep = stream_handle.play_once(BufReader::new(file)).unwrap();
    beep.set_volume(0.2);
    println!("Started beep");
    thread::sleep(Duration::from_millis(1500));
}
