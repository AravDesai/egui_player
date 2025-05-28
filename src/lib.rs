use av_format::stream;
use core::panic;
use cpal;
use eframe::{
    egui::{
        self, Align, Color32, Context, Pos2, ProgressBar, Rect, Response, Sense, Slider, Ui, Vec2,
    },
    glow::ProgramBinary,
};
use mp3_duration;
use rodio::{self, Decoder, OutputStreamHandle, Sink, source::Source};
use std::{
    fs::{self, File},
    io::BufReader,
    mem::discriminant,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Formats duration into a String with HH:MM:SS or MM:SS depending on inputted duration
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

/// Checks file extension of passed in file path to determine if it is an audio or video file
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

/// Gets the duration of a particular media
fn get_total_time(media_type: MediaType, file_path: &str) -> Duration {
    match media_type {
        MediaType::Audio => {
            let file = BufReader::new(File::open(file_path).unwrap());

            // The 2 lines below are rodio currently. Finding a way to get duration with cpal
            let source = Decoder::new(file).unwrap();
            Source::total_duration(&source)
                .unwrap_or(mp3_duration::from_path(file_path).unwrap_or(Duration::ZERO))
        }
        MediaType::Video => todo!(),
        MediaType::Error => panic!("Can not get time because of unsupported format"),
    }
}

/// Checks for presence of audio and returns relevant AudioPlayer if detected
fn get_audio_player(media_type: MediaType, file_path: &str) -> AudioPlayer {
    match media_type {
        MediaType::Audio => {
            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let file = File::open(file_path).unwrap();
            let bufReader = BufReader::new(file);
            AudioPlayer {
                stream_handle,
                display_volume: 1.0,
                thread_volume: Arc::new(AtomicI32::new(100)),
            }
        }
        MediaType::Video => todo!(),
        MediaType::Error => todo!(),
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlayerState {
    Playing,
    Paused,
    Ended,
}

pub struct AudioPlayer {
    display_volume: f32,
    thread_volume: Arc<AtomicI32>,
    stream_handle: OutputStreamHandle,
}

pub struct MediaPlayer {
    pub media_type: MediaType,
    pub player_size: Vec2,
    pub player_scale: f32,
    pub player_state: PlayerState,
    pub elapsed_time: Duration,
    pub total_time: Duration,
    pub playback_guard: bool,
    pub stopwatch_rx: Option<Receiver<Duration>>,
    pub stop_playback: Arc<AtomicBool>,
    pub start_playback_threadless: bool,
    pub stop_playback_threadless: bool,
    pub audio_player: AudioPlayer,
    pub file_path: String,
}

impl MediaPlayer {
    /// Initializes the player
    /// Use the MediaPlayer.ui() function to display it
    pub fn new(file_path: &str) -> Self {
        // gets relevant information that can only be taken from the filepath
        let media_type = get_media_type(file_path);
        let total_time = get_total_time(media_type, file_path);
        let audio_player = get_audio_player(media_type, file_path);
        Self {
            media_type,
            player_size: Vec2::default(),
            player_state: PlayerState::Paused,
            elapsed_time: Duration::ZERO,
            total_time,
            player_scale: 1.0,
            playback_guard: false,
            stopwatch_rx: None,
            stop_playback: Arc::new(AtomicBool::new(false)),
            audio_player,
            file_path: file_path.to_string(),
            start_playback_threadless: false,
            stop_playback_threadless: true,
        }
    }

    /// Allows you to rescale the player
    pub fn set_player_scale(&mut self, scale: f32) {
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

    fn transcribe_audio(&mut self) {
        println!("Currently under development");
    }

    /// Displays bar containing pause/play, video time, draggable bar and volume control
    fn control_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let pause_icon = match self.player_state {
                PlayerState::Playing => "⏸",
                PlayerState::Paused => "▶",
                PlayerState::Ended => "↺",
            };
            if ui.button(pause_icon).clicked() {
                match self.player_state {
                    // Pausing the player
                    PlayerState::Playing => {
                        self.pause_player();
                    }
                    // Playing the player
                    PlayerState::Paused => {
                        self.play_player();
                    }
                    // Restarting the player
                    PlayerState::Ended => {
                        self.elapsed_time = Duration::ZERO;
                        self.play_player();
                    }
                }
            }

            if self.elapsed_time >= self.total_time {
                self.pause_player();
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
                self.pause_player();
            }
            if slider_response.dragged() {
                self.elapsed_time = Duration::from_secs_f32(slider_value);
            }

            let volume_icon = if self.audio_player.display_volume > 0.7 {
                "🔊"
            } else if self.audio_player.display_volume > 0.4 {
                "🔉"
            } else if self.audio_player.display_volume > 0. {
                "🔈"
            } else {
                "🔇"
            };

            ui.menu_button(volume_icon, |ui| {
                ui.add(Slider::new(&mut self.audio_player.display_volume, 0.0..=1.0).vertical())
            });

            self.audio_player.thread_volume.store(
                (self.audio_player.display_volume * 100.0) as i32,
                Ordering::Relaxed,
            );

            ui.menu_button("…", |ui| {
                if ui.button("Transcribe audio").clicked() {
                    self.transcribe_audio();
                }
            });
        });
    }

    // TODO fix this eventually
    fn display_player(&mut self, ui: &mut Ui) {
        match self.media_type {
            MediaType::Audio => self.control_bar(ui),
            MediaType::Video => self.control_bar(ui),
            MediaType::Error => panic!("Can't display due to invalid file type"),
        }
    }

    fn audio_stream(&mut self) {
        if self.playback_guard {
            let start_at = self.elapsed_time;
            let file_path = self.file_path.clone();
            let stop_audio = Arc::clone(&self.stop_playback);
            let audio_volume = Arc::clone(&self.audio_player.thread_volume);
            thread::spawn(move || {
                let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
                let file = File::open(file_path).unwrap();
                let sink = stream_handle.play_once(BufReader::new(file)).unwrap();
                sink.try_seek(start_at).unwrap();
                loop {
                    sink.set_volume(audio_volume.load(Ordering::Acquire) as f32 / 100.0);
                    if stop_audio.load(Ordering::Relaxed) {
                        break;
                    }
                }
            });
        }
    }

    /// Starts visual/ audio stream by redirecting to the correct function
    fn start_stream(&mut self) {
        match self.media_type {
            MediaType::Audio => self.audio_stream(),
            MediaType::Video => todo!(),
            MediaType::Error => todo!(),
        }
    }

    fn stop_stopwatch(&mut self) {
        self.stop_playback.swap(true, Ordering::Relaxed);
    }

    fn start_stopwatch(&mut self) {
        self.stopwatch_rx = None;
        self.playback_guard = true;
        self.stop_playback = Arc::new(AtomicBool::new(false));
    }

    fn play_player(&mut self) {
        self.player_state = PlayerState::Playing;
        //self.start_stopwatch();
        self.start_stream();
    }

    fn pause_player(&mut self) {
        self.player_state = PlayerState::Paused;
        self.stop_stopwatch();
    }

    /// Sets up stopwatch that the play bar follows. Thread creation is guarded by stopwatch_guard
    fn setup_stopwatch(&mut self, ctx: Context) {
        //let mut receiver_option: Option<Receiver<Duration>> = None;
        if self.playback_guard {
            self.playback_guard = false;
            let (tx, rx) = channel();
            self.stopwatch_rx = Some(rx);
            let start_time = self.elapsed_time;
            let end_time = self.total_time;
            let stop_stopwatch = Arc::clone(&self.stop_playback);
            thread::spawn(move || {
                let start_instant = Instant::now();
                loop {
                    let _ = tx.send(start_instant.elapsed() + start_time);
                    if start_instant.elapsed() >= end_time || stop_stopwatch.load(Ordering::Relaxed)
                    {
                        break;
                    }
                    ctx.request_repaint();
                    thread::sleep(Duration::from_millis(10));
                }
            });
        }

        self.elapsed_time = match &self.stopwatch_rx {
            Some(received) => match received.recv() {
                Ok(time) => time,
                Err(_) => self.elapsed_time,
            },
            None => self.elapsed_time,
        }
    }

    // fn get_elapsed_time(&mut self) {
    //     if self.start_playback_threadless {}
    // }

    /// Responsible for initializing all values in self and then for displaying the player
    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        self.set_player_scale(self.player_scale);
        let (rect, response) = ui.allocate_exact_size(self.player_size, Sense::click());
        if ui.is_rect_visible(rect) {
            self.setup_stopwatch(ui.ctx().clone());
            //self.get_elapsed_time();
            self.display_player(ui);
        }
        response
    }

    /// Call this to show the player on screen
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        self.add_contents(ui)
    }
}
