<div align="center"><img alt="shellcaster logo: Ferris the crab with headphones" src="img/shellcaster-logo_smol.png"/></div>

# Note

This is a leisurely maintained fork of the original
[shellcaster](https://github.com/jeff-hughes/shellcaster/), which is now
unmaintained. Bugs will be fixed and minor changes will happen, but there should
not be any new major features. For a more featureful fork, you might be
interested in [hullcaster](https://github.com/gilcu3/hullcaster).

This project is hosted on
[Gitlab](https://gitlab.com/alpou/shellcaster-classic) and mirrored on
[Github](https://github.com/Elcoid/shellcaster-classic). Issues and pull
requests are accepted on both platforms. However, in case of mismatch between
the two, consider the version on Gitlab as the correct one.

# Shellcaster

Shellcaster is a terminal-based podcast manager, built in Rust. It provides a
terminal UI (i.e., an ncurses-like interface) to allow users to subscribe to
podcast feeds, and sync feeds to check for new episodes. Episodes may be
downloaded locally, played with an external media player, and marked as
played/unplayed. Keybindings and other options are configurable via a config
file.

<div align="center"><img alt="screenshot of shellcaster" src="img/screenshot.png"/></div>

## Installing shellcaster

### On Linux

Make sure to have some dependencies installed. The required packages
on Arch-based distributions are the following:

* git
* gcc
* pkgconf
* rust
* sqlite

Notes:

* The names of the packages on other distributions will likely be different.
  For `sqlite`, you are looking for the development headers for SQLite, which
  may be separate from the runtime package (e.g., `libsqlite3-dev` on Debian-
  based systems).
* If you enable the "native_tls" feature of shellcaster (disabled by default),
  you will also need `libssl-dev`, the development headers for OpenSSL (not
  needed on MacOS).
* If you enable the "sqlite-bundled" feature of shellcaster (disabled by
  default), `pkgconf` and `sqlite` are not necessary.

Next, download and compile the source code:

```bash
git clone https://github.com/Elcoid/shellcaster-classic
cd shellcaster-classic
cargo build --release  # add or remove any features with --features

# to install for all users
sudo cp target/release/shellcaster /usr/local/bin/

# to install only for your user, without root permissions
# make sure that ~/.local/bin is in your PATH environment variable
cp target/release/shellcaster ~/.local/bin
```

See below for the list of available features when compiling.

### On *BSD and MacOS

Shellcaster is developed for Linux. However it should work out of the box
on *BSD and MacOS. Simply follow the same instructions as for Linux.

If you notice that something more is needed, please submit a patch or a pull
request.

### On other systems

Shellcaster is **not currently supported** on other systems.

Although some work has been done to try to get it working on Windows, unicode
support is weak and there are issues when resizing the screen. You *might* have
better luck using the new Windows Terminal, but this has not been tested.

If you are a user of Haiku, Redox, Windows, SerenityOS, Genode, HelenOS, Plan9,
or other systems and want to help work out the bugs, pull requests are welcome!

### List of compile features

By default, only the `native_certs` feature is enabled. Here is the full list of
features:

* `sqlite_bundled`: When disabled, Rust will try to link shellcaster with SQLite
  header files already present on your system. If enabled, Rust will instead
  build SQLite from source and bundle the program with shellcaster. Bundling
  results in a larger application size, but may be suitable if you wish to use a
  different version of SQLite than the one on your system, or if you are on a
  system where installing SQLite is more difficult.

* `native_tls`: By default, shellcaster uses the
  [rustls](https://crates.io/crates/rustls) crate to enable TLS support (i.e.,
  URLs with https). This may cause issues with some podcast feeds that use
  earlier versions of TLS (below TLS v1.2). If you find that some feeds are
  unable to update, you can try enabling the `native_tls` feature, which will
  instead use the [native-tls](https://crates.io/crates/native-tls) crate --
  which uses OpenSSL on Linux, Secure Transport on MacOS, and SChannel on
  Windows.

* `native_certs`: Shellcaster will use the trusted certificate roots from the
  trust store for your OS in order to validate TLS certificates. Turning this
  feature off will instead use a bundled copy of the Mozilla Root program, which
  will only be updated when you recompile shellcaster. Thus, leaving this
  feature enabled is recommended.

To specify different features when compiling, here is the format:

```bash
cargo install --no-track --no-default-features --features "<feature1>,<feature2>" --root "$HOME/.local"
```

The format is the same when using `cargo build` instead:

```bash
cargo build --release --no-default-features --features "<feature1>,<feature2>"
cp target/release/shellcaster ~/.local/bin/
```

## Running shellcaster

Easy peasy! In your terminal, run:

```bash
shellcaster
```

Note that if you installed shellcaster to a different location, ensure that this
location has been added to your `$PATH`:

```bash
export PATH="/path/to/add:$PATH"
```

## Importing/exporting podcasts

Shellcaster supports importing OPML files from other podcast managers. If you
can export to an OPML file from another podcast manager, you can import this
file with:

```bash
shellcaster import -f /path/to/OPML/file.opml
```

If the `-r` flag is added to this command, it will overwrite any existing
podcasts that are currently stored in shellcaster. You can also pipe in data to
`shellcaster import` from stdin by not specifying the `-f <file>`.

You can export an OPML file from shellcaster with the following command:

```bash
shellcaster export -f /path/to/output/file.opml
```

You can also export to stdout by not specifying the `-f <file>`; for example,
this command is equivalent:

```bash
shellcaster export > /path/to/output/file.opml
```

## Configuring shellcaster

If you want to change configuration settings, the sample `config.toml` file can
be copied from [here](config.toml). Download it, edit it to your fancy, and
place it in the following location:

```bash
# on Linux and *BSD
mkdir -p ~/.config/shellcaster
cp config.toml ~/.config/shellcaster/

# on MacOS
mkdir -p ~/Library/Preferences/shellcaster
cp config.toml ~/Library/Preferences/shellcaster/
```

Or you can put `config.toml` in a place of your choosing, and specify the
location at runtime:

```bash
shellcaster -c /path/to/config.toml
```

The sample file above provides comments that should walk you through all the
available options. If any field does not appear in the config file, it will be
filled in with the default value specified in those comments. The defaults are
also listed below, for convenience.

### Configuration options

**download_path**:
* Specifies where podcast episodes that are downloaded will be stored.
* Defaults:
  * On Linux: `$XDG_DATA_HOME/shellcaster/` or `$HOME/.local/share/shellcaster/`
  * On MacOS: `$HOME/Library/Application Support/shellcaster/`
  * On Windows: `C:\Users\`**username**`\AppData\Local\shellcaster\`

**play_command**:
* Command used to play episodes. Use "%s" to indicate where file/URL will be
  entered to the command. Note that shellcaster does *not* include a native
  media player -- it simply passes the file path/URL to the given command with
  no further checking as to its success or failure. This process is started
  *in the background*, so be sure to send it to a program that has GUI controls
  of some kind so you have control over the playback.
* Default: "vlc %s"

**download_new_episodes**:
* Configures what happens when new episodes are found as podcasts are synced.
  Valid options:
    * "always" will automatically download all new episodes;
    * "ask-selected" will open a popup window to let you select which episodes
      to download, with all of them selected by default;
    * "ask-unselected" will open a popup window to let you select with episodes
      to download, with none of them selected by default;
    * "never" will never automatically download new episodes.
* Default: "ask-unselected"

**simultaneous_downloads**:
* Maximum number of files to download simultaneously. Setting this too high
  could result in network requests being denied. A good general guide would be
  to set this to the number of processor cores on your computer.
* Default: 3

**max_retries**:
* Maximum number of times to retry connecting to a URL to sync a podcast or
  download an episode.
* Default: 3

#### Default keybindings

    Key       | Action
    --------- | --------------
    ?         | Open help window
    Arrows    | Navigate menus
    h,j,k,l   | Navigate menus
    Shift+K   | Up 1/4 page
    Shift+J   | Down 1/4 page
    PgUp      | Page up
    PgDn      | Page down
    a         | Add new feed
    q         | Quit program
    s         | Synchronize selected feed
    Shift+S   | Synchronize all feeds
    Enter / p | Play selected episode
    m         | Mark selected episode as played/unplayed
    Shift+M   | Mark all episodes as played/unplayed
    d         | Download selected episode
    Shift+D   | Download all episodes
    x         | Delete downloaded file
    Shift+X   | Delete all downloaded files
    r         | Remove selected feed/episode from list
    Shift+R   | Remove all feeds/episodes from list
    1         | Toggle played/unplayed filter
    2         | Toggle downloaded/undownloaded filter

**Note:** Actions can be mapped to more than one key (e.g., "Enter" and "p" both
play an episode), but a single key may not do more than one action (e.g., you
can't set "d" to both download and delete episodes).

#### Customizable colors

You can set the colors in the app with either built-in terminal colors or
(provided your terminal supports it) customizable colors as well. See the
"colors" section in the [config.toml](config.toml) for details about how to
specify these colors!

## Syncing without the UI

Some users may wish to sync their podcasts automatically on a regular basis,
e.g., every morning. The `shellcaster sync` subcommand can be used to do this
without opening up the UI, and does a full sync of all podcasts in the database.
This could be used to set up a cron job or systemd timer, for example. Please
refer to the relevant documentation for these systems for setting it up on the
schedule of your choice.

## Contributing

Contributions from others are welcome! If you wish to contribute, feel free to
clone the repo and submit pull requests. **Please ensure you are on the
`develop` branch when making your edits**, as this is where the continued
development of the app is taking place. Pull requests will only be merged to the
`develop` branch, so you can help to avoid merge conflicts by doing your work on
that branch in the first place.

Thanks to these fine folks who have made contributions:
[a-kenji](https://github.com/a-kenji),
[dougli1sqrd](https://github.com/dougli1sqrd),
[dwvisser](https://github.com/dwvisser),
[thunderbiscuit](https://github.com/thunderbiscuit)

## Why "shellcaster"?

The original author was trying to come up with a play on the word "podcast", and
he liked the use of the word "shell" for several reasons. "Shell" is a synonym
for the word "pod". The terminal is also referred to as a shell (and shellcaster
is a terminal-based program). In addition, the program is built in Rust, whose
mascot is Ferris the crab. Finally, he just personally enjoy that "shellcaster"
sounds a lot like "spellcaster", so you can feel like a wizard when you use the
program...


