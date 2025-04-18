use eframe::egui;

enum MediaType {
    Audio,
    Video,
}

pub struct MediaPlayer {
    media_type: MediaType,
}
