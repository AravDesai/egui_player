# A Player for Rust using Egui

## Playback files

https://github.com/user-attachments/assets/f2dc0ac1-1248-46c2-8619-f9e413a9c515

## Interactive Transcription

![transcript_demo](https://github.com/user-attachments/assets/4ebc03fa-229f-4143-a66b-c18395a6ddcc)

## Usage

First, add Player to your App variables. Insert the path to the file to be played in `new()`

New takes an enum called InputMode. This lets you play a file from a filepath (`String`) or with bytes (`Vec<u8>`)

```rust
struct MyApp {
    player: Player,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            player: Player::new(InputMode::FilePath("your_path".to_string())),
        }
    }
}
```

For bytes, enter the following line instead:

```rust
player: Player::new(InputMode::Bytes(your_bytes)),
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
