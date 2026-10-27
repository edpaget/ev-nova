ev-nova nightly build
=====================

A tester build of the latest `main` of https://github.com/edpaget/ev-nova.
Each download holds:

  nova        the game: opens the game data in a window
              (Nova.app on macOS, nova.exe on Windows)
  nova-dump   writes the game data out as JSON, PNG and WAV files
  README.txt  this file
  LICENSE     GPL-3.0-or-later; covers the code, not the game data

The downloads are:

  ev-nova-aarch64-apple-darwin.dmg      macOS (Apple silicon), a disk image
  ev-nova-x86_64-pc-windows-msvc.msi    Windows, an installer
  ev-nova-x86_64-pc-windows-msvc.zip    Windows, a portable copy
  ev-nova-x86_64-unknown-linux-gnu.tar.gz  Linux

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

More detail: the "Using your own data" section of the README,
https://github.com/edpaget/ev-nova#using-your-own-data
