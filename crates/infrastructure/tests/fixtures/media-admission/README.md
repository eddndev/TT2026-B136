# Synthetic media admission fixtures

These files were generated for the repository from FFmpeg color and sine-wave
sources. They contain no recordings, personal information, or external media.
The initial generator was FFmpeg 7.1.5, using the local `libmp3lame`,
`libopenh264`, AAC, PNG, MJPEG, and PCM encoders.

## Content recipes

The commands below describe the generated content and codec settings. The exact
original command lines were not retained. Encoder versions, defaults, build
options, and metadata can change output bytes, so these recipes do not promise
byte-for-byte reproduction. The hashes below identify the committed fixtures;
changing a generator does not justify silently replacing their expected bytes.

Run recipes only in a separate directory when deliberately regenerating fixtures:

```bash
ffmpeg -f lavfi -i color=blue:size=32x32 -frames:v 1 -c:v png tiny.png
ffmpeg -f lavfi -i color=blue:size=32x32 -frames:v 1 -c:v mjpeg tiny.jpg
ffmpeg -f lavfi -i sine=frequency=440:sample_rate=8000 -t 0.12 \
  -c:a libmp3lame -b:a 16k -write_xing 0 tiny.mp3
ffmpeg -f lavfi -i sine=frequency=440:sample_rate=8000 -t 0.12 \
  -c:a pcm_s16le tiny.wav
ffmpeg -f lavfi -i color=blue:size=32x32:rate=5 \
  -f lavfi -i sine=frequency=440:sample_rate=8000 -t 0.4 \
  -c:v libopenh264 -b:v 20k -c:a aac -b:a 16k tiny.mp4
ffmpeg -i tiny.mp4 -map 0 -c copy -movflags +faststart tiny-faststart.mp4
```

Both MP4 fixtures carry H.264 video and AAC audio. `tiny.mp4` places its `moov`
index after media data; `tiny-faststart.mp4` moves the index before media data.
They cover both seekable layouts without referring to external resources.

## Recorded file identities

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `tiny-faststart.mp4` | 2703 | `f0a0f9465388f96f9d40549680cb369e0a24462297cdf16be97339732a223191` |
| `tiny.jpg` | 230 | `7ffaed90a0456f204a5af7c9eceec650715e0802bf4f4b44394d897b2ae33896` |
| `tiny.mp3` | 620 | `e7d25e4d98f5b3c862492e5e064b6f5b75a245049e584df767187d337862b21c` |
| `tiny.mp4` | 2703 | `f071809cad8139c08a284488f441684cb198cd668f99b912efcc3ccbbf8029a7` |
| `tiny.png` | 117 | `d06b40259ba6bcfc5a85e6de0e1a8a142dcf461f26c99c3e19912ef7876265b4` |
| `tiny.wav` | 1998 | `e8b66e4575a6fce69c961490641aa4996d532e4736f4ce20e585a8ac92f35f3d` |

## Verification scope

Structural tests import these fixtures and derive malformed copies in memory;
they do not modify the files. Separate synthetic frame builders in the Rust
tests exercise framing and are not claims of decodable audio. Passing structural
inspection alone is insufficient for multimedia admission: the isolated decoder
must inspect the supported stream inventory and decode the complete file. These
fixtures supplement malformed-input and resource-limit tests; they are not an
exhaustive conformance corpus for their respective formats.
