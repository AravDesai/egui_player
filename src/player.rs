use core::panic;
use eframe::egui::{Label, Response, ScrollArea, Sense, Slider, Ui, Vec2};
use infer;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::File,
    io::{BufReader, Cursor},
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc,
    },
    thread::{self},
    time::{Duration, Instant},
};

use crate::{
    media_information, InputMode, MediaType, ModelPath, TranscriptionData, TranscriptionProgress,
    TranscriptionSettings,
};

/// Reflects the current form of the [`Player`]
///
/// Playing: The Player
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlayerState {
    Playing,
    Paused,
    Ended,
}

/// Holds relevant info to run the player
#[derive(Debug)]
pub struct Player {
    /// Meta data information
    pub media_type: MediaType,
    pub file_input: InputMode,

    /// Player settings
    pub player_size: Vec2,
    pub player_scale: f32,
    pub player_state: PlayerState,

    /// Info related to control bar
    pub elapsed_time: Duration,
    pub total_time: Duration,

    /// Playback information
    playback_guard: bool,
    start_playback: bool,
    stop_playback: Arc<AtomicBool>,
    stopwatch_instant: Option<Instant>,
    pub start_time: Duration,

    /// Audio related info
    pub volume: Arc<AtomicI32>,
    transcription_settings: TranscriptionSettings,
    pub transcript: Vec<TranscriptionData>,
    pub model_path: ModelPath,
    transcription_progress: TranscriptionProgress,
    transcript_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<TranscriptionProgress>>,
}

impl Player {
    /// Initializes the [`Player`]
    ///
    /// Takes an [`InputMode`]
    ///
    /// To initialize with a filepath:
    ///
    /// ``` rust
    /// Player::new(InputMode::FilePath("your_path_here".to_string()))
    /// ```
    /// Use the ``Player.ui()`` function to display it
    ///
    /// Look at the *[README](https://github.com/AravDesai/egui-player/blob/master/README.md)* to have a more in depth approach to adding a [`Player`] to your egui project
    /// Or look at the example in examples/main.rs
    pub fn from_path(file_path: &str) -> Self {
        Self::new(InputMode::FilePath(file_path.to_string()))
    }

    /// To initialize with bytes (``Vec<u8>``):
    ///
    /// ``` rust
    /// Player::new(InputMode::Bytes(your_bytes))
    /// ```
    /// Use the ``Player.ui()`` function to display it
    ///
    /// Look at the *[README](https://github.com/AravDesai/egui-player/blob/master/README.md)* to have a more in depth approach to adding a [`Player`] to your egui project
    /// Or look at the example in examples/main.rs
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::new(InputMode::Bytes(bytes))
    }

    /// Accepts
    fn new(file: InputMode) -> Self {
        // gets relevant information that can only be taken from the filepath
        let media_type = match file.clone() {
            InputMode::FilePath(file_path) => media_information::get_media_type(&file_path),
            InputMode::Bytes(bytes) => {
                if let Some(kind) = infer::get(&bytes) {
                    media_information::get_media_type(kind.extension())
                } else {
                    panic!("Invalid File")
                }
            }
        };
        Self {
            media_type,
            player_size: Vec2::default(),
            player_state: PlayerState::Paused,
            elapsed_time: Duration::ZERO,
            total_time: media_information::get_total_time(media_type, file.clone()),
            player_scale: 1.0,
            playback_guard: false,
            stop_playback: Arc::new(AtomicBool::new(false)),
            file_input: file,

            start_playback: false,
            stopwatch_instant: None,
            start_time: Duration::ZERO,
            volume: Arc::new(AtomicI32::new(100)),
            transcript: vec![],
            transcript_receiver: None,
            transcription_settings: TranscriptionSettings::None,
            transcription_progress: TranscriptionProgress::NoProgress,
            model_path: ModelPath::Default,
        }
    }

    /// Configure transcription settings by changing the [`TranscriptionSettings`] enum
    pub fn set_transcript_settings(&mut self, setting: TranscriptionSettings) {
        self.transcription_settings = setting;
    }

    /// Configure where model is downloaded
    pub fn set_model_download_path(&mut self, file_path: String) {
        self.model_path = ModelPath::Custom(file_path);
    }

    /// Allows you to rescale the player ``(Note: Currently non-functional)``
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
                media_information::format_duration(self.elapsed_time)
                    + " / "
                    + &media_information::format_duration(self.total_time),
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

            let is_timestamped = matches!(
                self.transcription_settings,
                TranscriptionSettings::ShowTimeStamps
            );

            match self.transcription_settings {
                TranscriptionSettings::None => {}
                TranscriptionSettings::Allow
                | TranscriptionSettings::TranscriptLabel
                | TranscriptionSettings::ShowTimeStamps => {
                    ui.menu_button("…", |ui| {
                        if ui.button("Transcribe audio").clicked()
                            && self.transcript_receiver.is_none()
                        {
                            self.transcription_progress = TranscriptionProgress::Reading;
                            let file_input = self.file_input.clone();
                            let model_path = self.model_path.clone();
                            let (tx_transcript, rx_transcript) =
                                tokio::sync::mpsc::unbounded_channel();
                            self.transcript_receiver = Some(rx_transcript);

                            tokio::spawn(async move {
                                let _ = media_information::transcribe_audio(
                                    file_input,
                                    is_timestamped,
                                    Some(tx_transcript),
                                    model_path,
                                )
                                .await;
                            });
                        }
                    });
                }
            }

            if let Some(receiver) = &mut self.transcript_receiver {
                if let Ok(progress) = receiver.try_recv() {
                    self.transcription_progress = progress;
                };

                match &self.transcription_progress {
                    TranscriptionProgress::NoProgress => {}
                    TranscriptionProgress::InProgress(transcription_data) => {
                        if !self.transcript.contains(transcription_data) {
                            self.transcript.push(transcription_data.clone());
                        }
                        ui.label("Transcription in Progress");
                        ui.spinner();
                    }
                    TranscriptionProgress::Reading => {
                        ui.label("Transcription in Progress");
                        ui.spinner();
                    }
                    TranscriptionProgress::Finished => {
                        self.transcript_receiver = None;
                    }
                };
            }
        });

        match self.transcription_settings {
            TranscriptionSettings::TranscriptLabel | TranscriptionSettings::ShowTimeStamps => {
                if !self.transcript.is_empty() {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 0.0;
                            for word in self.transcript.clone() {
                                let response = ui.add(Label::new(word.text).sense(Sense::click()));
                                if response.clicked() {
                                    self.pause_player();
                                    self.elapsed_time = word.time;
                                }
                            }
                        });
                        if self.transcription_progress == TranscriptionProgress::Finished {
                            ui.label("--- END OF TRANSCRIPT ---");
                        }
                    });
                }
            }
            _ => {}
        }
    }

    // TODO fix this eventually
    fn display_player(&mut self, ui: &mut Ui) {
        match self.media_type {
            MediaType::Audio => self.control_bar(ui),
            MediaType::Video => self.control_bar(ui),
            MediaType::Error => panic!("Can't display due to invalid file type"),
        }
    }

    /// Audio playback
    ///
    /// A stream to play audio is started. It is only stopped when the file reaches the end or the [`Player`] is paused
    fn audio_stream(&mut self) {
        if self.playback_guard {
            let start_at = self.elapsed_time;
            let file_input = self.file_input.clone();
            let stop_audio = Arc::clone(&self.stop_playback);
            let volume = Arc::clone(&self.volume);
            thread::spawn(move || {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let sink: Sink = match file_input {
                    InputMode::FilePath(file_path) => {
                        let file = File::open(file_path).unwrap();
                        stream_handle.play_once(BufReader::new(file)).unwrap()
                    }
                    InputMode::Bytes(bytes) => {
                        let sound_data: Arc<[u8]> = Arc::from(bytes);
                        let cursor = Cursor::new(Arc::clone(&sound_data));
                        let try_sink = Sink::try_new(&stream_handle).unwrap();
                        let source = Decoder::new(cursor).unwrap();
                        try_sink.append(source);
                        try_sink
                    }
                };
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

    /// Responsible for keeping track of [`elapsed_time`]
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