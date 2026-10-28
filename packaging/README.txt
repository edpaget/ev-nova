ev-nova nightly build
=====================

A tester build of the latest `main` of https://github.com/edpaget/ev-nova.
Each download holds:

  nova        the game: opens the game data in a window
              (Nova.app on macOS, nova.exe on Windows)
  nova-dump   writes the game data out as JSON, PNG and WAV files
  README.txt  this file
  LICENSE     GPL-3.0-or-later; covers the code, not the game data
  share/      Linux only: the desktop entry, AppStream metadata and icons

The downloads are:

  ev-nova-aarch64-apple-darwin.dmg      macOS (Apple silicon), a disk image
  ev-nova-x86_64-pc-windows-msvc.msi    Windows, an installer
  ev-nova-x86_64-pc-windows-msvc.zip    Windows, a portable copy
  ev-nova-x86_64-unknown-linux-gnu.tar.gz   Linux, a folder to copy
  ev-nova-x86_64-unknown-linux-gnu.AppImage Linux, one file to run
  ev-nova-x86_64-unknown-linux-gnu.flatpak  Linux, a Flatpak to install

EV Nova's data files are copyrighted and are not included. Supply your own
copy of the game's `Nova Files` directory.

Running
-------

  nova "<path>/Nova Files"
      or set NOVA_DATA to the `Nova Files` directory and run `nova`.

  nova-dump "<path>/Nova Files" <out-dir> [--plugins <dir>]
      Pass the `Nova Files` directory itself, not the folder that contains
      it. `nova-dump --help` lists the options.

On Windows the programs are `nova.exe` and `nova-dump.exe`.

macOS
-----

The macOS download is a disk image (.dmg) holding Nova.app, nova-dump,
README.txt and LICENSE. It is for Apple silicon Macs and needs macOS 13
(Ventura) or later.

Open the .dmg and drag Nova.app onto the Applications folder beside it.
It is named Nova.app, not EV Nova.app, so it never replaces the original
game there. Double-click it to play: it opens the `Nova Files` it used
last time, else those of an `EV Nova.app` in /Applications or
~/Applications, and failing those it asks you to locate your EV Nova
folder, and remembers your pick. It logs its startup to
~/Library/Logs/nova/nova.log.

Nova.app and nova-dump are signed with a Developer ID and notarized by
Apple, so Nova.app opens with a double-click, and nova-dump copied out of
the .dmg runs from a terminal as it is.

A build made without the signing keys (a local `mise run bundle`) is
ad-hoc signed instead: macOS blocks a downloaded copy of it until you
choose Open Anyway in System Settings > Privacy & Security, or run
`xattr -dr com.apple.quarantine /Applications/Nova.app`.

Windows
-------

The installer (.msi) installs the game for you alone, without
administrator rights, into %LOCALAPPDATA%\Programs\Nova. It adds a "Nova"
entry to the Start menu and an "EV Nova" entry to Settings > Apps >
Installed apps (Apps & features), where you uninstall it. Uninstalling
removes the program but keeps your settings and pilots in
%APPDATA%\nova. Installing a newer nightly over an older one replaces it
in place.

The zip is a portable copy: unzip it anywhere and run nova.exe. Nothing
is installed, and deleting the folder removes it.

Either way the game finds your EV Nova's `Nova Files` itself, or asks you
for your EV Nova folder and remembers your pick.

Neither is signed yet, so SmartScreen may warn about an unknown publisher
when you open the installer or nova.exe. Choose "More info", then "Run
anyway".

Linux
-----

Every Linux download is for x86-64 (64-bit Intel or AMD). There are
three: the tarball, the AppImage and the Flatpak.

The tarball and the AppImage need glibc 2.35 or newer: Ubuntu 22.04 or
later, Debian 12 or later, or a current Fedora or openSUSE. They are
built on Ubuntu 22.04, and the build checks that the game needs no newer
glibc. They play sound through your system's ALSA, which PipeWire or
PulseAudio serves on current desktops, and share one set of settings and
pilots. The Flatpak brings its own runtime, so any distribution with
Flatpak runs it, and keeps its own settings and pilots (see below).

The tarball (.tar.gz) holds one folder with nova, nova-dump, README.txt,
LICENSE and a share/ folder:

  share/applications/io.github.edpaget.ev-nova.desktop
      the menu entry: what launchers, docks and app grids show
  share/metainfo/io.github.edpaget.ev-nova.metainfo.xml
      AppStream metadata, which software centres read
  share/icons/hicolor/<size>/apps/io.github.edpaget.ev-nova.png
      the icon, at each size from 16x16 to 512x512

To install it for you alone, from inside the unpacked folder:

  mkdir -p ~/.local/bin ~/.local/share
  cp nova nova-dump ~/.local/bin/
  cp -r share/applications share/icons share/metainfo ~/.local/share/
  update-desktop-database ~/.local/share/applications   (optional)

The menu entry starts `nova` from your PATH, so ~/.local/bin must be on
the PATH your desktop session starts with (most distributions put it there
once it exists; log out and in again). If it isn't, edit the Exec= line of
~/.local/share/applications/io.github.edpaget.ev-nova.desktop to the full
path of nova. "EV Nova" then appears among your applications, and the
running game shows its icon and name in the dock and task switcher.

Without installing, run ./nova from the unpacked folder.

The game opens the `Nova Files` it used last time, else those of the
Windows release in a Wine prefix ($WINEPREFIX, ~/.wine, or Lutris's
~/Games/ev-nova, under drive_c/Program Files or Program Files (x86)),
else a copied folder at ~/.local/share/nova/Nova Files
($XDG_DATA_HOME/nova) or ~/Games/EV Nova/Nova Files. Failing those it
asks you for your EV Nova folder through the desktop's file chooser, and
remembers your pick. Started from the menu, it shows a startup error or
warnings in a box when zenity is installed; they are always in the
startup log.

The game keeps your settings in ~/.config/nova ($XDG_CONFIG_HOME/nova),
saves your pilots in ~/.local/share/nova/Pilots ($XDG_DATA_HOME/nova),
and logs its startup to ~/.local/state/nova/nova.log
($XDG_STATE_HOME/nova). Earlier builds saved pilots in
~/.config/nova/Pilots: the first launch of this one moves them, every
file, and says so in its startup log. A pilot whose name is already taken
in the new folder is left where it was, with a warning.

The AppImage
------------

The AppImage is the game in one file, with nothing to install:

  chmod +x ev-nova-x86_64-unknown-linux-gnu.AppImage
  ./ev-nova-x86_64-unknown-linux-gnu.AppImage

It needs FUSE to run (fusermount3, which most desktops have; it doesn't
need the older libfuse2). Without it, run it with
--appimage-extract-and-run. It installs nothing itself: to add it to your
applications, use an AppImage integration tool such as Gear Lever or
AppImageLauncher, which installs its desktop entry and icon. Without
one, some docks (GNOME on Wayland among them) show a generic icon for its
window. It finds the `Nova Files` and keeps its settings, pilots and log
in the same places as the tarball above, and shares them with it.
nova-dump is in the tarball only.

The Flatpak
-----------

The Flatpak runs the game in a sandbox on the freedesktop runtime. To
install it for you alone:

  flatpak install --user ev-nova-x86_64-unknown-linux-gnu.flatpak

It offers to fetch the runtime from Flathub if you don't have it. Then
start "EV Nova" from your applications, or run
`flatpak run io.github.edpaget.ev-nova`. Uninstall it with
`flatpak uninstall --user io.github.edpaget.ev-nova`.

The sandbox lets it show its window (Wayland, or X11 where there is no
Wayland) and use your GPU and sound. It can read, and not change, three
folders where the game data may be: ~/.wine, ~/Games/ev-nova (Lutris)
and ~/Games/EV Nova. It can't see the rest of your files, has no network,
and can't talk to other programs. If your EV Nova folder is anywhere
else (another Wine prefix, Bottles, Proton), the game asks for it through
your desktop's file chooser, which gives it that folder alone, and it
remembers the pick across launches and reboots. Its startup error and
warning boxes use the runtime's zenity.

The Flatpak keeps everything in ~/.var/app/io.github.edpaget.ev-nova/:
settings in config/nova, pilots in data/nova/Pilots and the startup log
in .local/state/nova/nova.log. These are separate from the tarball's and
the AppImage's: pilots are neither shared nor moved between them. To use
the same pilots, copy their files from one Pilots folder to the other.

Checking the icon (by hand)
---------------------------

After installing as above, on GNOME (Wayland) and in an X11 session:
start "EV Nova" from the applications list, and check that the dock (or
the top bar's app menu) and the Alt+Tab switcher show the game's icon, a
gold star in a ring on dark blue, and the name "EV Nova", not a generic
icon or "nova".

More detail: the "Using your own data" section of the README,
https://github.com/edpaget/ev-nova#using-your-own-data
