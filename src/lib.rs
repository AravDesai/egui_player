use core::panic;
use cpal;
use eframe::{
    egui::{
        self, Align, Color32, Context, Pos2, ProgressBar, Rect, Response, Sense, Slider, Ui, Vec2,
    },
    glow::ProgramBinary,
};
use rodio::{self, Decoder, source::Source};
use std::{
    fs::{self, File},
    io::BufReader,
    mem::discriminant,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use timer::{Guard, Timer};

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    if hours >= 1 {
        format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
    } else {
        format!("{:0>2}:{:0>2}", minutes, seconds)
    }
}

fn get_media_type(file_path: &str) -> MediaType {
    match Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some(ext) => match ext.to_lowercase().as_str() {
            "mp4" | "avi" | "mov" | "mkv" => MediaType::Video,
            "mp3" | "wav" | "flac" => MediaType::Audio,
            _ => MediaType::Error,
        },
        None => MediaType::Error,
    }
}

fn get_total_time(media_type: MediaType, file_path: &str) -> Duration {
    match media_type {
        MediaType::Audio => {
            let file = BufReader::new(File::open(file_path).unwrap());

            // The 2 lines below are rodio currently. Finding a way to get duration with cpal
            let source = Decoder::new(file).unwrap();
            Source::total_duration(&source).unwrap()
        }
        MediaType::Video => todo!(),
        MediaType::Error => panic!("Can not get time because of unsupported format"),
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

#[derive(Debug, Copy, Clone)]
pub enum PlayerState {
    Playing,
    Paused,
    Ended,
}

pub struct MediaPlayer {
    pub media_type: MediaType,
    pub player_size: Vec2,
    pub player_scale: f32,
    pub player_state: PlayerState,
    pub elapsed_time: Duration,
    pub total_time: Duration,
    pub timer: Timer,
    pub timer_guard: Option<Guard>,
}

impl MediaPlayer {
    /// Initializes the player
    /// Use the MediaPlayer.ui() function to display it
    pub fn new(file_path: &str) -> Self {
        // gets relevant information that can only be taken from the filepath
        let media_type = get_media_type(file_path);
        let total_time = get_total_time(media_type, file_path);
        Self {
            media_type,
            player_size: Vec2::default(), // Vec2 { x: 0.0, y: 0.0 },
            player_state: PlayerState::Paused,
            elapsed_time: Duration::ZERO,
            total_time,
            player_scale: 1.0,
            timer: timer::Timer::new(),
            timer_guard: None,
        }
    }

    /// Allows you to rescale the player
    // TODO maybe rename to set_player_scale
    pub fn set_player_size(&mut self, scale: f32) {
        self.player_scale = scale;
        if self.player_size.eq(&Vec2::default()) {
            match self.media_type {
                MediaType::Audio => {
                    self.player_size = Vec2 { x: 50.0, y: 10.0 } * self.player_scale
                }
                MediaType::Video => self.player_size = Vec2 { x: 0.0, y: 0.0 } * self.player_scale,
                MediaType::Error => panic!("No size since it is an unsupported type"),
            }
        } else {
            self.player_size = self.player_size * self.player_scale;
        }
    }

    /// Displays bar containing pause/play, video time, draggable bar and volume control
    fn control_bar_display(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let pause_icon = match self.player_state {
                PlayerState::Playing => "⏸",
                PlayerState::Paused => "▶",
                PlayerState::Ended => "↺",
            };
            if ui.button(pause_icon).clicked() {
                match self.player_state {
                    PlayerState::Playing => self.player_state = PlayerState::Paused,
                    PlayerState::Paused => self.player_state = PlayerState::Playing,
                    PlayerState::Ended => self.player_state = PlayerState::Playing,
                }
            }

            if self.elapsed_time >= self.total_time {
                self.player_state = PlayerState::Ended;
            }

            ui.label(
                format_duration(self.elapsed_time) + " / " + &format_duration(self.total_time),
            );

            let mut slider_value = self.elapsed_time.as_secs_f32();
            let slider = Slider::new(&mut slider_value, 0.0..=self.total_time.as_secs_f32())
                .show_value(false);
            let slider_response = ui.add(slider);
            if slider_response.drag_started() {
                self.player_state = PlayerState::Paused;
            }
            if slider_response.dragged() {
                self.elapsed_time = Duration::from_secs_f32(slider_value);
            }

            // let audio_volume_frac = self.options.audio_volume.get() / self.options.max_audio_volume;
            // let sound_icon = if audio_volume_frac > 0.7 {
            //     "🔊"
            // } else if audio_volume_frac > 0.4 {
            //     "🔉"
            // } else if audio_volume_frac > 0. {
            //     "🔈"
            // } else {
            //     "🔇"
            // };

            ui.menu_button("…", |ui| {
                if ui.button("Transcribe audio").clicked() {
                    println!("Feature still in development");
                }
            });
        });
    }

    // TODO fix this eventually
    fn display_player(&mut self, ui: &mut Ui) {
        match self.media_type {
            MediaType::Audio => self.control_bar_display(ui),
            MediaType::Video => self.control_bar_display(ui),
            MediaType::Error => panic!("Can't display due to invalid file type"),
        }
    }

    fn audio_stream(&mut self) {}

    fn start_stream(&mut self) {
        match self.media_type {
            MediaType::Audio => todo!(),
            MediaType::Video => todo!(),
            MediaType::Error => todo!(),
        }
    }

    /// Responsible for initializing all values in self and then for displaying the player
    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        self.set_player_size(self.player_scale);
        let (rect, response) = ui.allocate_exact_size(self.player_size, Sense::click());
        if ui.is_rect_visible(rect) {
            //self.start_stream();
            self.display_player(ui);
        }
        response
    }

    /// Call this to show the player on screen
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        self.add_contents(ui)
    }
}
