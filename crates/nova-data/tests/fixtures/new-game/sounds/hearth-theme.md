---
placeholder: true
file: media/export/sounds/hearth-theme.mp3
licence: CC0-1.0
author: ev-nova
---

The game's music: eight silent MPEG-1 Layer III frames (each the header
`FF FB 90 64`, 128 kbit/s at 44.1 kHz, then 413 zero bytes; 3336 bytes in
all), written once for ev-nova's tests with no encoder, by Python's
standard library alone:

    python3 -I -c 'import sys; open(sys.argv[1], "wb").write((bytes([0xFF,0xFB,0x90,0x64]) + bytes(413)) * 8)' hearth-theme.mp3
