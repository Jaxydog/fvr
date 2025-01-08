# fvr

fvr (pronounced as 'fever') is an implementation of the [`ls`][1] command-line application.

fvr aims to be as fast and efficient as possible while still being "reasonably customizable" on a best-effort basis.
It does *not* make any attempt to replicate the command-line interface of [`ls`][1].

Currently, fvr is only intended for usage on Unix-based systems.
All code is currently developed and tested on Ubuntu-24.04, though this may change in the future.

## Installation

fvr can be installed through one of the following methods.

### Download the latest release

fvr's latest releases will be available through [this repository's 'releases' section][2].
These pre-compiled binaries will (for now) only be available for Unix-based systems.

### Install through Cargo

You can install fvr directly through [Cargo][3], the package manager for The Rust Programming Language.
This will download, compile, and then install fvr directly from this repository.

```sh
cargo install --git https://github.com/Jaxydog/fvr.git --locked
```

### Install manually

You may alternatively download fvr's source code directly, compile, and install it yourself.

```sh
git clone https://github.com/Jaxydog/fvr.git
cd ./fvr
cargo build --release
cp ./target/release/fvr <destination>
```

## Usage

fvr currently contains two sub-commands; `list` and `tree`.

`list` prints the contents of any specified directories.
`tree` prints the contents of any specified directories recursively as a tree.

```
$ fvr --help

fvr v0.1.0
  An implementation of the `ls` command-line application.

Usage: fvr [SUBCOMMAND] [ARGUMENTS]

Sub-commands:
  list                           List the contents of directories
  tree                           List the contents of directories in a tree-based view

Arguments:
  -h, --help                     Shows the command's usage
  -V, --version                  Shows the command's version
      --color [CHOICE]           Determines whether to output with color
                                 - default: auto
                                 - options: auto, always, never
```

## License

fvr is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

fvr is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with fvr. If not,
see <https://www.gnu.org/licenses/>.

[1]: https://pubs.opengroup.org/onlinepubs/9699919799/utilities/ls.html
[2]: https://github.com/Jaxydog/fvr/releases
[3]: https://doc.rust-lang.org/cargo/
