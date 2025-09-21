use core::panic;
use futures_util::stream::StreamExt;
use kalosm_sound::Whisper;
use rodio::{source::Source, Decoder};
use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
    time::Duration,
};

use crate::{InputMode, MediaType, ModelPath, TranscriptionData, TranscriptionProgress};

/// Formats [`Duration`] into a [`String`] with HH:MM:SS or MM:SS depending on inputted [`Duration`]
///
/// # Examples
///
/// ``` rust
/// use egui_player::media_information;
/// use std::time::Duration;
///
/// let formatted_duration = media_information::format_duration(Duration::from_secs(64))
///
/// ```
/// This would return 01:04
///
/// ``` rust
/// use egui_player::media_information;
/// use std::time::Duration;
///
/// let formatted_duration = media_information::format_duration(Duration::from_secs(5422))
///
/// ```
/// This would return 01:30:22
pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    if hours >= 1 {
        format!("{hours:0>2}:{minutes:0>2}:{seconds:0>2}")
    } else {
        format!("{minutes:0>2}:{seconds:0>2}")
    }
}

/// Checks file extension of passed in file path / extension to determine if it is an audio or video file
/// # Examples
///
/// ``` rust
/// use egui_player::media_information;
///
/// let media_type = media_information::get_media_type("hello.mp3")
///
/// ```
/// This would return ``MediaType::Audio``
pub fn get_media_type(file_path: &str) -> MediaType {
    let mut ext = Some(file_path);
    if file_path.contains(".") {
        ext = Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str());
    }

    match ext {
        Some(extenstion) => match extenstion.to_lowercase().as_str() {
            "mp4" | "avi" | "mov" | "mkv" => MediaType::Video,
            "mp3" | "wav" | "m4a" | "flac" => MediaType::Audio,
            _ => MediaType::Error,
        },
        None => MediaType::Error,
    }
}

/// Gets the length of a supported media in [`Duration`] format
///
/// For supported types, look at the *[README](https://github.com/AravDesai/egui-player/blob/master/README.md)*
pub fn get_total_time(media_type: MediaType, input_mode: InputMode) -> Duration {
    match media_type {
        MediaType::Audio => {
            let mut duration: Duration = match input_mode {
                InputMode::FilePath(file_path) => {
                    let file = BufReader::new(File::open(&file_path).unwrap());
                    match Path::new(&file_path)
                        .extension()
                        .and_then(|ext| ext.to_str())
                    {
                        Some(ext) => match ext.to_lowercase().as_str() {
                            "mp3" => mp3_duration::from_path(file_path).unwrap_or(Duration::ZERO),
                            _ => {
                                let source = Decoder::new(file).unwrap();
                                Source::total_duration(&source).unwrap_or(Duration::ZERO)
                            }
                        },
                        None => Duration::ZERO,
                    }
                }
                InputMode::Bytes(bytes) => {
                    if let Some(kind) = infer::get(&bytes) {
                        let ext = kind.extension();
                        match ext {
                            "mp3" => mp3_duration::from_read(&mut Cursor::new(bytes))
                                .unwrap_or(Duration::ZERO),
                            _ => {
                                if let Ok(decoder) = Decoder::new(Cursor::new(bytes)) {
                                    decoder.total_duration().unwrap_or(Duration::ZERO)
                                } else {
                                    Duration::ZERO
                                }
                            }
                        }
                    } else {
                        Duration::ZERO
                    }
                }
            };

            if duration != Duration::ZERO {
                duration += Duration::from_secs(1);
            }
            duration
        }
        MediaType::Video => todo!(),
        MediaType::Error => panic!("Can not get time because of unsupported format"),
    }
}

/// Transcribes audio and returns a Vec of [`TranscriptionData`] which contains a segment of words and its associated start time
///
/// You can pass in true for ``is_timestamped`` for it to include start and end times in text segments
///
/// ``progress_sender`` is relevant for Player use [`None`] if using it outside of it's context
///
/// ``model_path`` is relevant for custom installation of the model. Use [`ModelPath::Default`] if you want to run it with default installation path
///
/// # Examples
///
/// ``` rust
/// use egui_player::{media_information, ModelPath};
///
/// let transcript = media_information::transcribe_audio("hello.mp3", true, None, ModelPath::Default);
///
/// ```
/// This would return MediaType::Audio
pub async fn transcribe_audio(
    file_input: InputMode,
    is_timestamped: bool,
    progress_sender: Option<tokio::sync::mpsc::UnboundedSender<TranscriptionProgress>>,
    model_path: ModelPath,
) -> Vec<TranscriptionData> {
    let model = Whisper::new().await.unwrap();
    let mut text_stream;
    let mut transcript: Vec<TranscriptionData> = vec![];

    match file_input {
        InputMode::FilePath(file_path) => {
            let file = BufReader::new(File::open(file_path).unwrap());
            let audio = Decoder::new(file).unwrap();
            text_stream = model.transcribe(audio).timestamped();
        }
        InputMode::Bytes(bytes) => {
            let cursor = Cursor::new(bytes);
            let audio = Decoder::new(cursor).unwrap();
            text_stream = model.transcribe(audio).timestamped();
        }
    };

    let mut segment_counter = 0.0;

    while let Some(segment) = text_stream.next().await {
        for chunk in segment.chunks() {
            if let Some(time_range) = chunk.timestamp() {
                let true_start = time_range.start + (30.0 * segment_counter);
                let true_end = time_range.end + (30.0 * segment_counter);
                let transcription_data = TranscriptionData {
                    text: {
                        if is_timestamped {
                            format!(
                                "{}-{}: {}\n",
                                format_duration(Duration::from_secs_f32(true_start)),
                                format_duration(Duration::from_secs_f32(true_end)),
                                chunk
                            )
                        } else {
                            format!("{chunk}")
                        }
                    },
                    time: Duration::from_secs_f32(true_start),
                };
                if let Some(ref progress) = progress_sender {
                    let _ = progress.send(TranscriptionProgress::InProgress(
                        transcription_data.clone(),
                    ));
                }
                transcript.push(transcription_data);
            }
            if let Some(ref progress) = progress_sender {
                let _ = progress.send(TranscriptionProgress::Reading);
            }
        }
        segment_counter += 1.0;
    }
    if let Some(progress) = progress_sender {
        let _ = progress.send(TranscriptionProgress::Finished);
    }
    transcript
}
