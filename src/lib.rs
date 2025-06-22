use av_format::stream;
use core::panic;
use cpal;
use eframe::egui::{Response, Sense, Slider, Ui, Vec2};
use futures_util::{FutureExt, stream::StreamExt};
use kalosm_sound::{
    Whisper,
    rodio::{Decoder, OutputStream, source::Source},
};
use mp3_duration;
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    thread::{self},
    time::{Duration, Instant},
};
use tokio;

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

pub async fn transcribe_audio(file_path: &str) -> String {
    let model = Whisper::new().await.unwrap();
    let file = BufReader::new(File::open(file_path).unwrap());
    let audio = Decoder::new(file).unwrap();
    let mut text_stream = model.transcribe(audio);
    let mut transcript = String::new();
    while let Some(segment) = text_stream.next().await {
        for chunk in segment.chunks() {
            if let Some(ts) = chunk.timestamp() {
                transcript.push_str(&format!("{:.2}-{:.2}: {}\n", ts.start, ts.end, chunk));
            } else {
                transcript.push_str(&format!("{}", chunk));
            }
        }
    }
    return transcript;
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

#[derive(Debug)]
pub struct MediaPlayer {
    // Meta data information
    pub media_type: MediaType,
    pub file_path: String,

    // Player settings
    pub player_size: Vec2,
    pub player_scale: f32,
    pub player_state: PlayerState,

    // Info related to control bar
    pub elapsed_time: Duration,
    pub total_time: Duration,

    // Playback information
    pub playback_guard: bool,
    pub start_playback: bool,
    pub stop_playback: Arc<AtomicBool>,
    pub stopwatch_instant: Option<Instant>,
    pub start_time: Duration,

    // Audio related info
    pub volume: Arc<AtomicI32>,
    pub transcript: Option<String>,
    pub transcript_receiver: Option<tokio::sync::mpsc::Receiver<String>>,
}

impl MediaPlayer {
    /// Initializes the player
    /// Use the MediaPlayer.ui() function to display it
    pub fn new(file_path: &str) -> Self {
        // gets relevant information that can only be taken from the filepath
        let media_type = get_media_type(file_path);
        Self {
            media_type,
            player_size: Vec2::default(),
            player_state: PlayerState::Paused,
            elapsed_time: Duration::ZERO,
            total_time: get_total_time(media_type, file_path),
            player_scale: 1.0,
            playback_guard: false,
            stop_playback: Arc::new(AtomicBool::new(false)),
            file_path: file_path.to_string(),

            start_playback: false,
            stopwatch_instant: None,
            start_time: Duration::ZERO,
            volume: Arc::new(AtomicI32::new(100)),
            transcript: None,
            transcript_receiver: None,
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
            self.player_size *= self.player_scale;
        }
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

            let mut volume = self.volume.load(Ordering::Acquire);

            let volume_icon = if volume > 70 {
                "🔊"
            } else if volume > 40 {
                "🔉"
            } else if volume > 0 {
                "🔈"
            } else {
                "🔇"
            };

            ui.menu_button(volume_icon, |ui| {
                ui.add(Slider::new(&mut volume, 0..=100).vertical())
            });

            self.volume.store(volume, Ordering::Relaxed);

            ui.menu_button("…", |ui| {
                if ui.button("Transcribe audio").clicked() {
                    self.transcript_receiver = None;
                    let file_path = self.file_path.clone();
                    let (tx, rx) = tokio::sync::mpsc::channel(1);
                    self.transcript_receiver = Some(rx);
                    tokio::spawn(async move {
                        let transcription = transcribe_audio(&file_path).await;
                        println!("{}", transcription);
                        let _ = tx.send(transcription).await;
                    });
                }
            });

            if let Some(receiver) = &mut self.transcript_receiver {
                if let Some(potential_transcript) = receiver.recv().now_or_never() {
                    if let Some(transcript) = potential_transcript {
                        self.transcript = Some(transcript);
                        self.transcript_receiver = None;
                    }
                }
            }
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
            let volume = Arc::clone(&self.volume);
            thread::spawn(move || {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let file = File::open(file_path).unwrap();
                let sink = stream_handle.play_once(BufReader::new(file)).unwrap();
                sink.try_seek(start_at).unwrap();
                loop {
                    sink.set_volume(volume.load(Ordering::Acquire) as f32 / 100.0);
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

    fn play_player(&mut self) {
        self.player_state = PlayerState::Playing;
        self.start_playback = true;
        self.playback_guard = true;
        self.stop_playback = Arc::new(AtomicBool::new(false));
        self.start_stream();
    }

    fn pause_player(&mut self) {
        self.player_state = PlayerState::Paused;
        self.start_playback = false;
        self.stop_playback.swap(true, Ordering::Relaxed);
    }

    fn get_elapsed_time(&mut self) -> Duration {
        match self.stopwatch_instant {
            Some(instant) => instant.elapsed() + self.start_time,
            None => self.elapsed_time,
        }
    }

    fn setup_stopwatch(&mut self) {
        self.elapsed_time = self.get_elapsed_time();
        if self.start_playback {
            self.stopwatch_instant = Some(Instant::now());
            self.start_time = self.elapsed_time;
            self.start_playback = false;
        }
        if self.stop_playback.as_ref().load(Ordering::Acquire) {
            self.stopwatch_instant = None;
        }
    }

    /// Responsible for initializing all values in self and then for displaying the player
    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        self.set_player_scale(self.player_scale);
        let (rect, response) = ui.allocate_exact_size(self.player_size, Sense::click());
        if ui.is_rect_visible(rect) {
            self.setup_stopwatch();
            self.display_player(ui);
            ui.ctx().request_repaint_after(Duration::from_millis(10));
        }
        response
    }

    /// Call this to show the player on screen
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        self.add_contents(ui)
    }
}
