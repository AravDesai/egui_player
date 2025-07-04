use core::panic;
use eframe::egui::{Label, ProgressBar, Response, ScrollArea, Sense, Slider, Ui, Vec2};
use futures_util::{FutureExt, stream::StreamExt};
use kalosm_sound::{
    Whisper,
    rodio::{Decoder, OutputStream, source::Source},
};
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
            "mp3" | "wav" => MediaType::Audio,
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
            let mut duration = Source::total_duration(&source)
                .unwrap_or(mp3_duration::from_path(file_path).unwrap_or(Duration::ZERO));
            if duration != Duration::ZERO {
                duration += Duration::from_secs(1);
            }
            duration
        }
        MediaType::Video => todo!(),
        MediaType::Error => panic!("Can not get time because of unsupported format"),
    }
}

pub async fn transcribe_audio(
    file_path: &str,
    is_timestamped: bool,
    progress_sender: Option<tokio::sync::mpsc::Sender<f32>>,
) -> Vec<TranscriptionData> {
    let model = Whisper::new().await.unwrap();
    let file = BufReader::new(File::open(file_path).unwrap());
    let audio = Decoder::new(file).unwrap();
    let mut text_stream;
    let mut transcription_data: Vec<TranscriptionData> = vec![];

    text_stream = model.transcribe(audio).timestamped();
    let mut segment_counter = 0.0;

    while let Some(segment) = text_stream.next().await {
        for chunk in segment.chunks() {
            if let Some(time_range) = chunk.timestamp() {
                let true_start = time_range.start + (30.0 * segment_counter);
                let true_end = time_range.end + (30.0 * segment_counter);
                transcription_data.push(TranscriptionData {
                    text: {
                        if is_timestamped {
                            format!(
                                "{}-{}: {}\n",
                                format_duration(Duration::from_secs_f32(true_start)),
                                format_duration(Duration::from_secs_f32(true_end)),
                                chunk
                            )
                        } else {
                            format!("{}", chunk)
                        }
                    },
                    time: Duration::from_secs_f32(true_start),
                });
            }
        }
        segment_counter += 1.0;
        if let Some(ref tx) = progress_sender {
            let _ = tx.send(segment_counter).await;
        };
    }
    transcription_data
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TranscriptionSettings {
    None,
    Allow,
    TranscriptLabel,
    ShowTimeStamps,
}

#[derive(Debug, Clone)]
pub struct TranscriptionData {
    pub text: String,
    pub time: Duration,
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
    playback_guard: bool,
    start_playback: bool,
    stop_playback: Arc<AtomicBool>,
    stopwatch_instant: Option<Instant>,
    pub start_time: Duration,

    // Audio related info
    pub volume: Arc<AtomicI32>,
    transcription_settings: TranscriptionSettings,
    pub transcript: Option<Vec<TranscriptionData>>,
    transcript_receiver: Option<tokio::sync::mpsc::Receiver<Vec<TranscriptionData>>>,
    transcription_progress: f32,
    transcription_progress_receiver: Option<tokio::sync::mpsc::Receiver<f32>>,
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
            transcription_settings: TranscriptionSettings::None,
            transcription_progress: 0.0,
            transcription_progress_receiver: None,
        }
    }

    pub fn set_transcript_settings(&mut self, setting: TranscriptionSettings) {
        self.transcription_settings = setting;
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

            let is_timestamped = matches!(
                self.transcription_settings,
                TranscriptionSettings::ShowTimeStamps
            );

            ui.menu_button("…", |ui| match self.transcription_settings {
                TranscriptionSettings::None => {}
                TranscriptionSettings::Allow
                | TranscriptionSettings::TranscriptLabel
                | TranscriptionSettings::ShowTimeStamps => {
                    if ui.button("Transcribe audio").clicked() && self.transcript_receiver.is_none()
                    {
                        let file_path = self.file_path.clone();
                        let (tx_transcript, rx_transcript) = tokio::sync::mpsc::channel(1);
                        self.transcript_receiver = Some(rx_transcript);

                        let (tx_transcription_progress, rx_transcription_progress) =
                            tokio::sync::mpsc::channel(1);
                        self.transcription_progress_receiver = Some(rx_transcription_progress);

                        tokio::spawn(async move {
                            let transcription_data = transcribe_audio(
                                &file_path,
                                is_timestamped,
                                Some(tx_transcription_progress),
                            )
                            .await;
                            let _ = tx_transcript.send(transcription_data).await;
                        });
                    }
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

            if let Some(receiver) = &mut self.transcription_progress_receiver {
                ui.add(ProgressBar::new(self.transcription_progress).text(
                    "Transcription in Progress: ".to_string()
                        + &(self.transcription_progress * 100.0).to_string()
                        + "%",
                ));
                if let Some(potential_progress) = receiver.recv().now_or_never() {
                    if let Some(progress) = potential_progress {
                        self.transcription_progress =
                            (progress * 30.0) / self.total_time.as_secs_f32();
                    }
                }
                if self.transcription_progress >= self.total_time.as_secs_f32() {
                    self.transcription_progress_receiver = None;
                }
            }
        });

        match self.transcription_settings {
            TranscriptionSettings::TranscriptLabel | TranscriptionSettings::ShowTimeStamps => {
                if self.transcript.is_some() {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 0.0;
                            for word in self.transcript.clone().unwrap() {
                                let response = ui.add(Label::new(word.text).sense(Sense::click()));
                                if response.clicked() {
                                    self.pause_player();
                                    self.elapsed_time = word.time;
                                }
                            }
                        });
                        ui.label("--- END OF TRANSCRIPT ---");
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
