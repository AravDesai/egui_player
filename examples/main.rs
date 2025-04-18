use ffmpeg_next::device::input;
use ffmpeg_next::{self as ffmpeg, format};

use eframe::{self, NativeOptions};
use eframe::{App, egui};
use egui::CentralPanel;
use ffmpeg::format::input;
use rodio;
use show_image::{self, Image, ImageInfo, ImageView, WindowOptions};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::thread;
use std::time::Duration;
use video_rs::encode::Settings;
use video_rs::{self, Decoder, Encoder};

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
    println!("video_rs");
    show_image::run_context(|| {
        video_rs_no_audio();
    });
    // let _ = eframe::run_native(
    //     "app",
    //     NativeOptions::default(),
    //     Box::new(|_| Ok(Box::new(MyApp::default()))),
    // );
}

fn video_rs_no_audio() {
    let window = show_image::create_window("Video Player", WindowOptions::default()).unwrap();

    let mut decoder = Decoder::new(Path::new("assets/Greetings - Halo 2.mp4")).unwrap();
    let mut curr_frame = 0;
    let fps = decoder.frame_rate();

    decoder
        .decode_raw_iter()
        .take_while(Result::is_ok)
        .for_each(|frame| {
            curr_frame += 1;
            let clean_frame = frame.unwrap();

            let rgb_data = clean_frame.data(0);

            window
                .set_image(
                    curr_frame.to_string(),
                    ImageView::new(
                        ImageInfo::rgb8(clean_frame.width(), clean_frame.height()),
                        rgb_data,
                    ),
                )
                .unwrap();

            thread::sleep(Duration::from_millis((1000.0 / fps) as u64));
        });
}

fn ffmpeg_audio_test() {
    ffmpeg::init().unwrap();
    let unclean_beep = input("assets/beep.wav").unwrap();
    let beep = unclean_beep
        .streams()
        .best(ffmpeg::util::media::Type::Audio)
        .ok_or(ffmpeg::Error::StreamNotFound);

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
