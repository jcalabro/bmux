# bmux

tmux for bsky

## Install

### Prerequisites

- [Rust](https://rustup.rs/) (1.85+)
- [just](https://github.com/casey/just) (task runner)
- `libchafa` (for inline images)

```bash
# Arch
sudo pacman -S chafa

# Ubuntu/Debian
sudo apt install libchafa-dev

# macOS
brew install chafa
```

### Build

```bash
git clone https://github.com/jcalabro/bmux.git
cd bmux
just release
```

The binary lands in `target/release/bmux`.

## Usage

```bash
# Login with flags
bmux -u your.handle -p your-app-password

# Or use environment variables
export BMUX_IDENTIFIER="your.handle"
export BMUX_PASSWORD="your-app-password"
bmux
```

Use an [App Password](https://bsky.app/settings/app-passwords) — not your main password.

## Development

```bash
just build          # debug build
just test           # run tests
just lint           # clippy
just fmt            # format
just ci             # all checks
```

## License

MIT
