Mypass - a keepass database manager written in Rust
===========================

`Mypass` is a simple password manager written in [Rust](https://www.rust-lang.org/).
It's a GUI application that uses [egui](https://github.com/emilk/egui) framework and [keepass-rs](https://github.com/sseemayer/keepass-rs) library.

`Mypass` is a work in progress. It's not ready for daily use yet.

## Building

### Prerequisites

- [Rust](https://rustup.rs)
- Dependencies for [egui on Linux](https://github.com/emilk/egui#demo)
 
   * On Ubuntu: `sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`
   * On Fedora: `dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

- There needn't be any dependencies on Windows and macOS.

### Building

```bash
git clone https://github.com/ssrlive/mypass.git && cd mypass
cargo build --release
```

## Running

```bash
cargo run --release
```
or
```bash
./target/release/mypass
```

## Screenshots

![img](https://github.com/ssrlive/mypass/assets/30760636/4ded1594-e0d8-4ed1-ba18-233ab0e87f08)
