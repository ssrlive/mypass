# Mypass - a keepass database manager written in Rust

`Mypass` is a simple password manager written in [Rust](https://www.rust-lang.org/).
It's a GUI application that uses [wxDragon](https://crates.io/crates/wxdragon) and [keepass-ng](https://crates.io/crates/keepass-ng).

`Mypass` is a work in progress. It's not ready for daily use yet.

## Building

### Prerequisites

- [Rust](https://rustup.rs)
- wxWidgets development dependencies required by wxDragon.

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

![img](https://github.com/user-attachments/assets/6cb6369c-07b7-4049-8186-16ae5f943452)
