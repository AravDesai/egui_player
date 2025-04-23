use eframe::egui;
use rodio;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

#[derive(Debug)]
pub struct MediaPlayer {
    pub media_type: MediaType,
}

impl MediaPlayer {
    pub fn new(file_path: String) -> Self {
        let media_type = Self::get_media_type(file_path);
        Self { media_type }
    }

    fn get_media_type(file_path: String) -> MediaType {
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
}
