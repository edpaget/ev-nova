ev-nova nightly build
=====================

An unsigned tester build of the latest `main` of
https://github.com/edpaget/ev-nova. This archive holds:

  nova        opens the game data in a window (ship browser, galaxy map)
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

macOS: the binaries are unsigned, so Gatekeeper blocks them. In the
extracted folder, run

  xattr -d com.apple.quarantine nova nova-dump

or right-click each binary in Finder and choose Open.

Windows: SmartScreen may warn about an unknown publisher. Choose
"More info", then "Run anyway".

More detail: the "Using your own data" section of the README,
https://github.com/edpaget/ev-nova#using-your-own-data
