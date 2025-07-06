use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

/// Configure how transcript is outputted
///
/// ``None`` : No transcript field in Player
///
/// ``Allow``: Transcript field in Player
///
/// ``TranscriptLabel``: Transcript field in Player and inbuilt label
///
/// ``ShowTimeStamps``: Transcript field in Player and inbuilt label with start and stop timestamps
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TranscriptionSettings {
    None,
    Allow,
    TranscriptLabel,
    ShowTimeStamps,
}

/// Sent out for transcript Progress
///
/// ``NoProgress`` : No transcript started
///
/// ``InProgress(TranscriptionData)``: Words are being sent back
///
/// ``ReadingWords``: Nothing is being sent back but words are being read
///
/// ``Finished``: Done with Transcription
#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptionProgress {
    NoProgress,
    InProgress(TranscriptionData),
    ReadingWords,
    Finished,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionData {
    pub text: String,
    pub time: Duration,
}

pub mod media_information;
pub mod player;
