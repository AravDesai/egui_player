use eframe::egui::{self, Pos2, Rect, Response, Sense, Ui, Vec2};
use rodio;
use std::{
    fs,
    mem::discriminant,
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
    pub player_size: Vec2,
}

impl MediaPlayer {
    pub fn new(file_path: &str) -> Self {
        let media_type = Self::get_media_type(file_path);
        Self {
            media_type,
            player_size: Vec2 { x: 0.0, y: 0.0 },
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

    pub fn set_player_size(&mut self, x: f32, y: f32) {
        self.player_size = Vec2 { x, y }
    }

    fn get_player_size(mut self, ui: &mut Ui) -> Vec2 {
        // TODO handle video and error properly
        match self.media_type {
            MediaType::Audio => {
                if self.player_size == (Vec2 { x: 0.0, y: 0.0 }) {
                    Vec2 { x: 20.0, y: 10.0 } // TODO implement logic based on available size or set default and let user only change scale
                } else {
                    self.player_size
                }
            }
            MediaType::Video => Vec2 { x: 0.0, y: 0.0 },
            MediaType::Error => panic!("No size since it is an unsupported type"),
        }
    }

    fn display_player(&mut self, ui: &mut Ui) {
        ui.button("▶");
    }

    fn add_contents(&mut self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.player_size, Sense::click());
        if ui.is_rect_visible(rect) {
            self.display_player(ui);
        }
        //response.widget_info(|| egui::WidgetInfo::slider(true, 100.0, "aa"));

        response
    }
}

impl egui::Widget for MediaPlayer {
    fn ui(mut self, ui: &mut Ui) -> Response {
        self.add_contents(ui)
    }
}
