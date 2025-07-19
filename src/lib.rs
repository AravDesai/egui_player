use std::time::Duration;

/// Used throughout [`player`] to determine visual elements and populate relevant [`player::Player`] struct fields
///
/// Is also used by [`media_information`] for relevant functions
///
/// Use [`media_information::get_media_type`] to get media_type of a particular file
#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

/// Used in Player::new() determines input
#[derive(Debug, Clone)]
pub enum InputMode {
    FilePath(String),
    Bytes(Vec<u8>),
}

/// Configure if a transcript is outputted and displayed
///
/// ``None`` : Transcript field in Player is marked as ``None`` and there will be no advanced option to transcribe audio
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
/// ``Reading``: Nothing is being sent back but words are being read
///
/// ``Finished``: Done with Transcription
#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptionProgress {
    NoProgress,
    InProgress(TranscriptionData),
    Reading,
    Finished,
}

/// Holds data produced when a file is transcribed
///
/// The ``text`` section is usually a word with a space and relevant punctuation detected
///
/// The ``time`` section is when this word has started
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionData {
    pub text: String,
    pub time: Duration,
}

/// Functions that populate data for [`player::Player`]
///
/// Functions from this module can also be used independently (refer to function documentation if you want to use these functions)
pub mod media_information;

/// Contains [`player::Player`] a struct that holds all info needed for the player to run
pub mod player;
