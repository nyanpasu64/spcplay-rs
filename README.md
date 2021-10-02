# spcplay-rs

**This project is dead.**

spcplay-rs was an attempt to write a .spc player in Rust with bookmarks/savestates. Unfortunately, I ran into paper cuts with UI frameworks (kas triggered rust-analyzer panics and egui has limited layout abilities), and the snes-apu crate (which promised to be a "highly-accurate emulator" for SNES audio) had multiple severe flaws (using `+= 1` for wrapping increment which panics on debug, treating volumes as unsigned rather than two's complement/surround, and likely undefined behavior from aliased `&mut`) plus likely other flaws I'm not yet aware of. So I am not continuing development of this project in Rust.

## Getting started

Start by clicking "Use this template" at https://github.com/emilk/egui_template/ or follow [these instructions](https://docs.github.com/en/free-pro-team@latest/github/creating-cloning-and-archiving-repositories/creating-a-repository-from-a-template).

`src/app.rs` contains a simple example app. This is just to give some inspiration - most of it can be removed if you like.

Make sure you are using the latest version of stable rust by running `rustup update`

### Testing locally

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra speech-dispatcher-devel libxkbcommon-devel pkg-config openssl-devel`

## Updating egui

As of 2021, egui is in active development with frequent releases with breaking changes. [egui_template](https://github.com/emilk/egui_template/) will be updated in lock-step to always use the latest version of egui.
