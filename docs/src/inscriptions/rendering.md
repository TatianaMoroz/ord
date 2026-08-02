Rendering
=========

Aspect Ratio
------------

Inscriptions should be rendered with a square aspect ratio. Non-square aspect
ratio inscriptions should not be cropped, and should instead be centered and
resized to fit within their container.

Maximum Size
------------

The `ord` explorer, used by [ordinals.com](https://ordinals.com/), displays
inscription previews with a maximum size of 576 by 576 pixels, making it a
reasonable choice when choosing a maximum display size.

Image Rendering
---------------

The CSS `image-rendering` property controls how images are resampled when
upscaled and downscaled.

When downscaling image inscriptions, `image-rendering: auto`, should be used.
This is desirable even when downscaling pixel art.

When upscaling image inscriptions other than AVIF, `image-rendering: pixelated`
should be used. This is desirable when upscaling pixel art, since it preserves
the sharp edges of pixels. It is undesirable when upscaling non-pixel art, but
should still be used for visual compatibility with the `ord` explorer.

When upscaling AVIF and JPEG XL inscriptions, `image-rendering: auto` should be
used. This allows inscribers to opt-in to non-pixelated upscaling for non-pixel
art inscriptions. Until such time as JPEG XL is widely supported by browsers,
it is not a recommended image format.

Content-Type Fallback
---------------------

A number of inscriptions on Bitcoin were created with the wrong `content_type`
field — typically `application/octet-stream` for a payload that is in fact a
glTF model, PNG image, MP4 video, and so on. The on-chain state is immutable
and cannot be amended.

To preserve user-facing rendering for these inscriptions, the `/preview/<id>`
endpoint inspects the first 4 KiB of the inscription body (transparently
decoding brotli when `content_encoding == "br"`) and upgrades the inferred
`Media` based on recognised magic-byte signatures. The behaviour is invoked
only when the stored `content_type` would otherwise resolve to
`Media::Unknown`; correctly-tagged inscriptions are routed exactly as before.

The following formats are detected, in this order:

| Bytes inspected                           | Format    | Rendered as              |
| ----------------------------------------- | --------- | ------------------------ |
| `glTF` at offset 0                        | GLB       | `Model` (model-viewer)   |
| `{` followed by `"asset"` and `"version"` | glTF JSON | `Model` (model-viewer)   |
| `\x89PNG\r\n\x1a\n` at offset 0           | PNG       | `Image(Pixelated)`       |
| `\xff\xd8\xff` at offset 0                | JPEG      | `Image(Pixelated)`       |
| `GIF87a` or `GIF89a` at offset 0          | GIF       | `Image(Pixelated)`       |
| `RIFF????WEBP`                            | WebP      | `Image(Pixelated)`       |
| `ftyp` at offset 4, MP4-family brand      | MP4       | `Video`                  |
| `OggS` at offset 0                        | Ogg       | `Audio`                  |
| `RIFF????WAVE`                            | WAV       | `Audio`                  |
| `fLaC` at offset 0                        | FLAC      | `Audio`                  |
| `%PDF-` at offset 0                       | PDF       | `Pdf`                    |

The MP4 matcher brand-filters to `{mp4*, iso*, avc1, M4V , dash}`. HEIC, HEIF,
and QuickTime share the ISO-BMFF `ftyp` header but are deliberately rejected
so they fall through to the unknown preview rather than being mis-promoted to
`Media::Video`.

No format is auto-upgraded to `Media::Iframe`. Untrusted HTML or SVG bytes
arriving under a wrong content-type label are intentionally left as
`Media::Unknown`, because rendering arbitrary inscriptions inside an iframe
would expand the renderable surface beyond the on-chain content-type contract
and carries CSP and sandbox implications.

The brotli decode step is bounded by a hard `.take(4 KiB)` cap, so a
maliciously crafted small compressed payload cannot trigger an unbounded
decompression. Encodings other than brotli are not decoded; the raw bytes are
sniffed as-is and will simply fail to match any rule.

This rescue layer applies only to `/preview/<id>`. The `/content/<id>`
endpoint continues to serve the original on-chain `Content-Type` header,
leaving downstream clients (`<model-viewer>`, `<img>`, `<video>`) to do their
own content sniffing if they wish. Embed, thumbnail, and oEmbed responses
are not covered.

Implementation: `Media::from_body_head` and the `SNIFF_TABLE` constant in
`src/inscriptions/media.rs`; `Inscription::sniffed_media` in
`src/inscriptions/inscription.rs`; one call site in the `/preview/<id>`
handler in `src/subcommand/server.rs`.
