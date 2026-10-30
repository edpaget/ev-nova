# Movie fixtures

Tiny QuickTime movies the decoder's tests play. Each was made once and
checked in; none is stock game data. They were made with Homebrew's
`ffmpeg` 9.0.2 (`9.0.2_1`) and its libx264 (x264 r3222) on 2026-10-29.
Another encoder version may not make the same bytes, so the tests check
the properties that matter (codec, profile, B-frames, size, frame count,
display order) rather than the bytes.

Every movie is 64x32 (2:1, like the stock 768x384 movies), 10 fps, with no
audio. Each frame is flat, its luma rising by 20 a frame (`geq` with
`lum='16+N*20'`), so a test can tell the frames apart and see their
order.

## `main-bframes.mov`

H.264 Main profile with B-frames (so the decoder reorders frames, and the
movie has a `ctts` atom, like `Race 1.mov` to `Race 4.mov`): 10 frames,
grey.

```sh
ffmpeg -f lavfi -i "nullsrc=size=64x32:rate=10,geq=lum='16+N*20':cb=128:cr=128,format=yuv420p" \
  -frames:v 10 -c:v libx264 -profile:v main -bf 2 -g 10 \
  -x264-params bframes=2:b-adapt=0:scenecut=0 -an -map_metadata -1 -f mov main-bframes.mov
```

`ffprobe` reports `profile=Main`, `has_b_frames=2`, and packets whose pts
and dts differ.

## The plug-in codecs

Older QuickTime codecs a plug-in's movies may use, 3 frames each, coloured
(`cb=90:cr=200`) so the conversion to RGBA is seen:

```sh
SRC="nullsrc=size=64x32:rate=10,geq=lum='16+N*20':cb=90:cr=200,format=yuv420p"
ffmpeg -f lavfi -i "$SRC" -frames:v 3 -c:v cinepak -pix_fmt rgb24    -an -map_metadata -1 -f mov cinepak.mov
ffmpeg -f lavfi -i "$SRC" -frames:v 3 -c:v qtrle   -pix_fmt rgb24    -an -map_metadata -1 -f mov qtrle.mov
ffmpeg -f lavfi -i "$SRC" -frames:v 3 -c:v rpza    -pix_fmt rgb555be -an -map_metadata -1 -f mov rpza.mov
ffmpeg -f lavfi -i "$SRC" -frames:v 3 -c:v smc     -pix_fmt pal8     -an -map_metadata -1 -f mov smc.mov
ffmpeg -f lavfi -i "$SRC" -frames:v 3 -c:v svq1    -pix_fmt yuv410p  -an -map_metadata -1 -f mov svq1.mov
```

They decode to rgb24 (cinepak, qtrle), rgb555 (rpza), pal8 (smc, with
the palette from the sample description) and yuv410p (svq1).

FFmpeg has no SVQ3 (Sorenson Video 3) encoder, so no SVQ3 fixture exists:
the decoder is configured in, and checked to be there, but no SVQ3 movie
is decoded in the tests.
