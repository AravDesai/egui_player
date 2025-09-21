# A Player for Rust using Egui

## THIS BRANCH IS FOR THE 2021 RUST EDITION AND EGUI VER. 0.28
## THIS BRANCH IS USED FOR [LOCKBOOK DEVELOPMENT](https://lockbook.net/)
### This branch's README and other documentation will not be updated

If you want to use this for your project use the most recent release on [crates.io](https://crates.io/crates/egui_player) or use this repository's [master branch](https://github.com/AravDesai/egui-player)

## Playback files

https://github.com/user-attachments/assets/f2dc0ac1-1248-46c2-8619-f9e413a9c515

## Interactive Transcription

![transcript_demo](https://github.com/user-attachments/assets/4ebc03fa-229f-4143-a66b-c18395a6ddcc)

## Usage

First, add Player to your App variables. Insert the path to the file to be played in `new()`

```rust
struct MyApp {
    player: Player,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            player: Player::new("assets/Dreamweaver.mp3"),
        }
    }
}
```

For transcriptions: Set up an async block to allow for asynchronous tokio functions

```rust
fn main() {
    let rt = Runtime::new().unwrap();
    let _ = rt.block_on(async {
        eframe::run_native(
            "Player Example",
            NativeOptions::default(),
            Box::new(|_| Ok(Box::new(MyApp::default()))),
        )
    });
}
```

Now, under the update function, add this line to display the player:

```rust
self.player.ui(ui);
```

For faster transcription, run with the `release` flag

## Examples

To play your own files, clone/download this repository and use:
`cargo run --example main --release`

## Supported Audio Formats

| Format | Playback | Transcription |
| :----: | :------: | :-----------: |
|  mp3   |    ✅    |      ✅       |
|  m4a   |    ✅    |      ✅       |
|  wav   |    ✅    |      ✅       |
|  flac  |    ✅    |      ❌       |

## Supported Video Formats

Currently working on support

### Credits

Dreamweaver.mp3 track in demo assets made by [@romms921](https://github.com/romms921)
