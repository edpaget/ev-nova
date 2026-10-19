ev-nova nightly build
=====================

A tester build of the latest `main` of https://github.com/edpaget/ev-nova.
Each download holds:

  nova        the game: opens the game data in a window
              (Nova.app on macOS)
  nova-dump   writes the game data out as JSON, PNG and WAV files
  LICENSE     GPL-3.0-or-later; covers the code, not the game data

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
README.txt and LICENSE. It needs macOS 13 (Ventura) or later, on Apple
silicon or Intel.

Open the .dmg and drag Nova.app onto the Applications folder beside it.
It is named Nova.app, not EV Nova.app, so it never replaces the original
game there. Double-click it to play: it opens the `Nova Files` it used
last time, else those of an `EV Nova.app` in /Applications or
~/Applications, and failing those it asks you to locate your EV Nova
folder, and remembers your pick. It logs its startup to
~/Library/Logs/nova/nova.log.

Nova.app is ad-hoc signed but not notarized, so macOS blocks a downloaded
copy the first time. Try to open it once, then choose Open Anyway in
System Settings > Privacy & Security, or run

  xattr -dr com.apple.quarantine /Applications/Nova.app

nova-dump still runs from a terminal; copy it out of the .dmg and clear
its quarantine the same way:

  xattr -d com.apple.quarantine nova-dump

Windows: SmartScreen may warn about an unknown publisher. Choose
"More info", then "Run anyway".

More detail: the "Using your own data" section of the README,
https://github.com/edpaget/ev-nova#using-your-own-data
