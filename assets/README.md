# Assets

This directory contains demo assets for agent-tui.

## Generating the Demo GIF

The demo GIF is generated using [VHS by Charm](https://github.com/charmbracelet/vhs).

### Install VHS

```bash
# macOS
brew install charmbracelet/tap/vhs

# Linux (apt)
sudo apt install ffmpeg
brew install charmbracelet/tap/vhs  # or download from releases

# From Go
go install github.com/charmbracelet/vhs@latest
```

### Generate the GIF

```bash
cd /path/to/agent-tui
vhs assets/demo.tape
```

This will create `assets/demo.gif` from the tape script.

### Tape Script Format

The `demo.tape` file is a VHS script that records terminal sessions deterministically. Key commands:

- `Output <file>` - Output file path
- `Set Theme "<theme>"` - Terminal theme
- `Type "<text>"` - Type text
- `Enter` - Press Enter
- `Sleep <duration>` - Wait

See [VHS documentation](https://github.com/charmbracelet/vhs#vhs-documentation) for full syntax.

## Updating the Demo

1. Edit `demo.tape` with your changes
2. Run `vhs assets/demo.tape` to regenerate
3. Test the GIF plays correctly
4. Commit both the tape and generated GIF
