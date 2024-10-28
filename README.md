# spotic 🎛️ 🎵

#### Control Spotify from the command line

![demo.gif](https://raw.githubusercontent.com/GHaxZ/spotic/refs/heads/master/.github/demo.gif)

"**spotic**" is a **Spotify controller for the command line**, which makes it easy to control Spotify using simple commands or create automated scripts with Spotify functionality.


## Features

- **Playback control**
  - Resume, pause, or toggle playback
  - Go to the next or previous song
  - Set shuffle mode to either on or off
  - Set repeat mode to either on, off or track
- **Volume control**
  - Increase, decrease, or set volume levels
- **Play content**
  - Play any type of content on Spotify
- **Search content**
  - Search for any type of content on Spotify
- **Access library**
  - Play playlists saved in your library
- **Output information**
  - Output the current song
- **Playback device control**
  - Set your current playback device
## Installation

Head to the [releases](https://github.com/GHaxZ/spotic/releases) page and search for the latest release.

You are presented with multiple ways to install spotic:

- **Shell script**
  - Useable on Linux and macOS
  - No extra software required
  - No automatic updates
- **Powershell script**
  - Usable on Windows
  - No extra software required
  - No automatic updates
- **Homebrew**
  - Usable on Linux and macOS
  - Extra software required ([homebrew](https://brew.sh/))
  - Automatic updates

Alternatively, if you have Rust installed, you can compile yourself:

```bash
cargo install --git https://github.com/GHaxZ/spotic.git
```

## Usage

### Command usage

**Pause playback**:

```bash
sc pause
```

**Resume playback**:

```bash
sc resume
```

**Toggle playback**:

```bash
sc toggle
```

**Toggle shuffle mode**:

```bash
sc shuffle
```

**Set shuffle to on/off**:

```bash
sc shuffle on/off
```

**Set volume percentage**:

```bash
sc volume 50
```

**Increase/decrease volume percentage**:

```bash
sc volume +10/-20
```

**Play first matching content of type track**:

```bash
sc play -t "never gonna give you up"
```

When playing, it is required to specify which type of content you want to play.

To see all available types, run `sc play -h`.

**Display matching results for artists and play selected item**:

```bash
sc search -A "rick astley"
```

When searching, it is required to specify which type of content you want to search for.

To see all available types, run `sc search -h`.

**Display library and play selection**:

```bash
sc library
```

**Play first matching item from library**:

```bash
sc library "lofi beats"
```

**Output current song**:

```bash
sc current
```

**Display available playback devices and set selected device**:

```bash
sc device
```

**Set first matching available playback device**:

```bash
sc device "my-laptop"
```

## Contributing

Contributions are always welcome!

Please make sure you somewhat **adhere to the codebase style** and **document your code**, especially in hard-to-understand areas.

Thanks!


## Feedback

In case you encounter any **issues** or have a **feature you want to see**, please [open a github issue](https://github.com/GHaxZ/spotic/issues/new). I'll do my best to fix things!
## License

This project is licensed under the [MIT](https://choosealicense.com/licenses/mit/) license.

