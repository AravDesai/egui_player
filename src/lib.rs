use eframe::egui::{self, Align, Pos2, ProgressBar, Rect, Response, Sense, Ui, Vec2};
use std::{
    fs,
    mem::discriminant,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Error,
}

#[derive(Debug, Copy, Clone)]
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

    /// Allows you to rescale the player
    // TODO maybe rename to set_player_scale
    pub fn set_player_size(&mut self, scale: f32) {
        if self.player_size == (Vec2 { x: 0.0, y: 0.0 }) {
            match self.media_type {
                MediaType::Audio => self.player_size = Vec2 { x: 400.0, y: 120.0 },
                MediaType::Video => self.player_size = Vec2 { x: 0.0, y: 0.0 },
                MediaType::Error => panic!("No size since it is an unsupported type"),
            }
        } else {
            self.player_size = self.player_size * scale;
        }
    }

    fn player_bar_display(&mut self, ui: &mut Ui) {
        let thing = {
            ui.button("test");
            ui.label("testing again");
        };
    }

    // TODO fix this eventually
    fn display_player(&mut self, ui: &mut Ui) {
        match self.media_type {
            MediaType::Audio => self.player_bar_display(ui),
            MediaType::Video => self.player_bar_display(ui),
            MediaType::Error => panic!("Can't display due to invalid file type"),
        }
    }

    /// Responsible for initializing all values in self and then for displaying the player
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
