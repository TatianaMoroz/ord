use {
  self::{ImageRendering::*, Language::*, Media::*},
  super::*,
  brotli::enc::backward_references::BrotliEncoderMode::{
    self, BROTLI_MODE_FONT as FONT, BROTLI_MODE_GENERIC as GENERIC, BROTLI_MODE_TEXT as TEXT,
  },
  mp4::{MediaType, Mp4Reader, TrackType},
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Media {
  Audio,
  Code(Language),
  Font,
  Iframe,
  Image(ImageRendering),
  Markdown,
  Model,
  Pdf,
  Text,
  Unknown,
  Video,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Language {
  Css,
  JavaScript,
  Json,
  Python,
  Yaml,
}

impl Display for Language {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Css => "css",
        Self::JavaScript => "javascript",
        Self::Json => "json",
        Self::Python => "python",
        Self::Yaml => "yaml",
      }
    )
  }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ImageRendering {
  Auto,
  Pixelated,
}

impl Display for ImageRendering {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Auto => "auto",
        Self::Pixelated => "pixelated",
      }
    )
  }
}

impl Media {
  #[rustfmt::skip]
  const TABLE: &'static [(&'static str, BrotliEncoderMode, Media, &'static [&'static str])] = &[
    ("application/cbor",            GENERIC, Unknown,          &["cbor"]),
    ("application/json",            TEXT,    Code(Json),       &["json"]),
    ("application/octet-stream",    GENERIC, Unknown,          &["bin"]),
    ("application/pdf",             GENERIC, Pdf,              &["pdf"]),
    ("application/pgp-signature",   TEXT,    Text,             &["asc"]),
    ("application/protobuf",        GENERIC, Unknown,          &["binpb"]),
    ("application/x-bittorrent",    GENERIC, Unknown,          &["torrent"]),
    ("application/x-javascript",    TEXT,    Code(JavaScript), &[]),
    ("application/yaml",            TEXT,    Code(Yaml),       &["yaml", "yml"]),
    ("audio/flac",                  GENERIC, Audio,            &["flac"]),
    ("audio/mpeg",                  GENERIC, Audio,            &["mp3"]),
    ("audio/ogg",                   GENERIC, Audio,            &[]),
    ("audio/ogg;codecs=opus",       GENERIC, Audio,            &["opus"]),
    ("audio/wav",                   GENERIC, Audio,            &["wav"]),
    ("font/otf",                    GENERIC, Font,             &["otf"]),
    ("font/ttf",                    GENERIC, Font,             &["ttf"]),
    ("font/woff",                   GENERIC, Font,             &["woff"]),
    ("font/woff2",                  FONT,    Font,             &["woff2"]),
    ("image/apng",                  GENERIC, Image(Pixelated), &["apng"]),
    ("image/avif",                  GENERIC, Image(Auto),      &["avif"]),
    ("image/gif",                   GENERIC, Image(Pixelated), &["gif"]),
    ("image/jpeg",                  GENERIC, Image(Pixelated), &["jpg", "jpeg"]),
    ("image/jxl",                   GENERIC, Image(Auto),      &["jxl"]),
    ("image/png",                   GENERIC, Image(Pixelated), &["png"]),
    ("image/svg+xml",               TEXT,    Iframe,           &["svg"]),
    ("image/webp",                  GENERIC, Image(Pixelated), &["webp"]),
    ("model/gltf+json",             TEXT,    Model,            &["gltf"]),
    ("model/gltf-binary",           GENERIC, Model,            &["glb"]),
    ("model/stl",                   GENERIC, Unknown,          &["stl"]),
    ("text/css",                    TEXT,    Code(Css),        &["css"]),
    ("text/html",                   TEXT,    Iframe,           &[]),
    ("text/html;charset=utf-8",     TEXT,    Iframe,           &["html"]),
    ("text/javascript",             TEXT,    Code(JavaScript), &["js", "mjs"]),
    ("text/markdown",               TEXT,    Markdown,         &[]),
    ("text/markdown;charset=utf-8", TEXT,    Markdown,         &["md"]),
    ("text/plain",                  TEXT,    Text,             &[]),
    ("text/plain;charset=utf-8",    TEXT,    Text,             &["txt"]),
    ("text/x-python",               TEXT,    Code(Python),     &["py"]),
    ("video/mp4",                   GENERIC, Video,            &["mp4"]),
    ("video/webm",                  GENERIC, Video,            &["webm"]),
  ];

  pub(crate) fn content_type_for_path(
    path: &Path,
  ) -> Result<(&'static str, BrotliEncoderMode), Error> {
    let extension = path
      .extension()
      .ok_or_else(|| anyhow!("file must have extension"))?
      .to_str()
      .ok_or_else(|| anyhow!("unrecognized extension"))?;

    let extension = extension.to_lowercase();

    if extension == "mp4" {
      Media::check_mp4_codec(path)?;
    }

    for (content_type, mode, _, extensions) in Self::TABLE {
      if extensions.contains(&extension.as_str()) {
        return Ok((*content_type, *mode));
      }
    }

    let mut extensions = Self::TABLE
      .iter()
      .flat_map(|(_, _, _, extensions)| extensions.first().cloned())
      .collect::<Vec<&str>>();

    extensions.sort();

    Err(anyhow!(
      "unsupported file extension `.{extension}`, supported extensions: {}",
      extensions.join(" "),
    ))
  }

  pub(crate) fn check_mp4_codec(path: &Path) -> Result<(), Error> {
    let f = File::open(path)?;
    let size = f.metadata()?.len();
    let reader = BufReader::new(f);

    let mp4 = Mp4Reader::read_header(reader, size)?;

    for track in mp4.tracks().values() {
      if let TrackType::Video = track.track_type()? {
        let media_type = track.media_type()?;
        if media_type != MediaType::H264 {
          return Err(anyhow!(
            "Unsupported video codec, only H.264 is supported in MP4: {media_type}"
          ));
        }
      }
    }

    Ok(())
  }

  /// Sniff the first bytes of an inscription body and return the most specific
  /// `Media` variant whose magic-byte matcher accepts the head, or `None` if
  /// no matcher recognises the bytes.
  ///
  /// Used to rescue inscriptions whose stored `content_type` is wrong — most
  /// commonly `application/octet-stream` for a payload that is actually a glTF
  /// model, PNG image, MP4 video, and so on. The on-chain state is never
  /// modified; this only changes which preview template the server chooses at
  /// display time.
  ///
  /// The caller must hand in already-decompressed bytes. The full pipeline
  /// (including the brotli decode step for inscriptions with
  /// `content_encoding == "br"`) lives in `Inscription::sniffed_media`.
  pub(crate) fn from_body_head(head: &[u8]) -> Option<Self> {
    Self::SNIFF_TABLE
      .iter()
      .find_map(|(matches, media)| matches(head).then_some(*media))
  }

  /// Magic-byte signatures consulted by `Self::from_body_head`.
  ///
  /// Each row is `(matcher, Media)`. Matchers must be cheap (no allocation,
  /// no I/O) and decisive — the table returns on the first accepting matcher.
  ///
  /// To add a new format: define a `matches_*` function below and append a row.
  /// Two invariants the table relies on:
  ///
  ///   - A matcher should only confirm formats whose `Media` variant is in
  ///     fact rendered by the preview templates. The MP4 matcher, for example,
  ///     brand-filters to exclude HEIC, HEIF, and QuickTime — bytes whose
  ///     ISO-BMFF header would otherwise fool a naive `ftyp` check but which
  ///     the project does not promise to render as video.
  ///   - Nothing here should promote to `Media::Iframe`. Auto-upgrading
  ///     unknown-tagged HTML or SVG into an iframe would expand the renderable
  ///     surface beyond the on-chain content-type contract, which carries
  ///     CSP and sandbox implications.
  const SNIFF_TABLE: &'static [(fn(&[u8]) -> bool, Self)] = &[
    (Self::matches_glb,       Self::Model),
    (Self::matches_gltf_json, Self::Model),
    (Self::matches_png,       Self::Image(Pixelated)),
    (Self::matches_jpeg,      Self::Image(Pixelated)),
    (Self::matches_gif,       Self::Image(Pixelated)),
    (Self::matches_webp,      Self::Image(Pixelated)),
    (Self::matches_mp4,       Self::Video),
    (Self::matches_ogg,       Self::Audio),
    (Self::matches_wav,       Self::Audio),
    (Self::matches_flac,      Self::Audio),
    (Self::matches_pdf,       Self::Pdf),
  ];

  /// glTF binary (GLB). The four-byte magic `glTF` is followed on disk by a
  /// little-endian version and total length, but those fields are not needed
  /// for the sniff.
  fn matches_glb(head: &[u8]) -> bool {
    head.starts_with(b"glTF")
  }

  /// glTF 2.0 JSON. Verifies a leading `{` (after optional ASCII whitespace)
  /// and the presence of both `"asset"` and `"version"` somewhere in the
  /// head, which the spec requires of every conforming glTF document.
  fn matches_gltf_json(head: &[u8]) -> bool {
    let trimmed = head
      .iter()
      .position(|b| !b.is_ascii_whitespace())
      .map(|i| &head[i..])
      .unwrap_or(&[]);
    if !trimmed.starts_with(b"{") {
      return false;
    }
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
      haystack.windows(needle.len()).any(|w| w == needle)
    }
    contains(head, b"\"asset\"") && contains(head, b"\"version\"")
  }

  /// PNG. Eight-byte fixed signature. APNG shares it and is rendered as
  /// `Image(Pixelated)` for consistency with the project's MIME table.
  fn matches_png(head: &[u8]) -> bool {
    head.starts_with(b"\x89PNG\r\n\x1a\n")
  }

  /// JPEG. Three-byte SOI marker (`\xff\xd8\xff`) followed by another segment
  /// marker (JFIF `\xe0`, Exif `\xe1`, DQT `\xdb`, Adobe APP14 `\xee`, ...),
  /// all of which are valid fourth bytes.
  fn matches_jpeg(head: &[u8]) -> bool {
    head.starts_with(b"\xff\xd8\xff")
  }

  /// GIF, either GIF87a or GIF89a.
  fn matches_gif(head: &[u8]) -> bool {
    head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a")
  }

  /// WebP. Bytes 0..4 = `RIFF`, bytes 8..12 = `WEBP`. The FourCC at offset 8
  /// is what distinguishes WebP from sibling RIFF containers like WAV or AVI.
  fn matches_webp(head: &[u8]) -> bool {
    head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP"
  }

  /// MP4 (ISO Base Media File Format), constrained to brands the project will
  /// actually render. Requires `ftyp` at offset 4 and a brand in the set
  /// `{mp4*, iso*, avc1, M4V , dash}`.
  ///
  /// Other ISO-BMFF brands — notably HEIC and HEIF (image, not video) and
  /// QuickTime (`qt  `, a codec the renderer does not claim to support) — are
  /// deliberately rejected so they fall through to `PreviewUnknownHtml`
  /// rather than being mis-promoted to `Media::Video`.
  fn matches_mp4(head: &[u8]) -> bool {
    if head.len() < 12 || &head[4..8] != b"ftyp" {
      return false;
    }
    let brand = &head[8..12];
    brand.starts_with(b"mp4")
      || brand.starts_with(b"iso")
      || brand == b"avc1"
      || brand == b"M4V "
      || brand == b"dash"
  }

  /// Ogg container. Always promoted to `Media::Audio` because the project's
  /// MIME table declares only audio Ogg variants. Theora-in-Ogg would
  /// mis-route here, but is not relevant to inscriptions in practice.
  fn matches_ogg(head: &[u8]) -> bool {
    head.starts_with(b"OggS")
  }

  /// WAV. Bytes 0..4 = `RIFF`, bytes 8..12 = `WAVE`. Shares the RIFF container
  /// with WebP and AVI; the FourCC at offset 8 disambiguates.
  fn matches_wav(head: &[u8]) -> bool {
    head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WAVE"
  }

  /// FLAC. Four-byte magic `fLaC` at offset 0. Files prefixed with an ID3v2
  /// header (rare for FLAC but permitted by some encoders) are not handled
  /// here; they would have to be rescued via a separate ID3 matcher.
  fn matches_flac(head: &[u8]) -> bool {
    head.starts_with(b"fLaC")
  }

  /// PDF. Five-byte signature `%PDF-`; the version digits follow (`1.4`,
  /// `1.7`, `2.0`, ...).
  fn matches_pdf(head: &[u8]) -> bool {
    head.starts_with(b"%PDF-")
  }
}

impl FromStr for Media {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    for entry in Self::TABLE {
      if entry.0 == s {
        return Ok(entry.2);
      }
    }

    Err(anyhow!("unknown content type: {s}"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn for_extension() {
    assert_eq!(
      Media::content_type_for_path(Path::new("pepe.jpg")).unwrap(),
      ("image/jpeg", BrotliEncoderMode::BROTLI_MODE_GENERIC)
    );
    assert_eq!(
      Media::content_type_for_path(Path::new("pepe.jpeg")).unwrap(),
      ("image/jpeg", BrotliEncoderMode::BROTLI_MODE_GENERIC)
    );
    assert_eq!(
      Media::content_type_for_path(Path::new("pepe.JPG")).unwrap(),
      ("image/jpeg", BrotliEncoderMode::BROTLI_MODE_GENERIC)
    );
    assert_eq!(
      Media::content_type_for_path(Path::new("pepe.txt")).unwrap(),
      (
        "text/plain;charset=utf-8",
        BrotliEncoderMode::BROTLI_MODE_TEXT
      )
    );
    assert_regex_match!(
      Media::content_type_for_path(Path::new("pepe.foo")).unwrap_err(),
      r"unsupported file extension `\.foo`, supported extensions: apng .*"
    );
  }

  #[test]
  fn h264_in_mp4_is_allowed() {
    assert!(Media::check_mp4_codec(Path::new("examples/h264.mp4")).is_ok(),);
  }

  #[test]
  fn av1_in_mp4_is_rejected() {
    assert!(Media::check_mp4_codec(Path::new("examples/av1.mp4")).is_err(),);
  }

  #[test]
  fn no_duplicate_extensions() {
    let mut set = HashSet::new();
    for (_, _, _, extensions) in Media::TABLE {
      for extension in *extensions {
        assert!(set.insert(extension), "duplicate extension `{extension}`");
      }
    }
  }

  #[test]
  fn sniff_glb_magic() {
    let mut body = b"glTF".to_vec();
    body.extend_from_slice(&[2, 0, 0, 0]);
    assert_eq!(Media::from_body_head(&body), Some(Media::Model));
  }

  #[test]
  fn sniff_gltf_json() {
    let body = br#"{
  "asset": { "version": "2.0" },
  "scene": 0
}"#;
    assert_eq!(Media::from_body_head(body), Some(Media::Model));
  }

  #[test]
  fn sniff_gltf_json_with_leading_whitespace() {
    let body = b"  \n  {\"asset\":{\"version\":\"2.1\"}}";
    assert_eq!(Media::from_body_head(body), Some(Media::Model));
  }

  #[test]
  fn sniff_random_bytes_stays_unknown() {
    let body = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03];
    assert_eq!(Media::from_body_head(&body), None);
  }

  #[test]
  fn sniff_json_without_asset_is_not_gltf() {
    let body = br#"{"foo":"bar"}"#;
    assert_eq!(Media::from_body_head(body), None);
  }

  #[test]
  fn sniff_png() {
    let body = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    assert_eq!(
      Media::from_body_head(body),
      Some(Media::Image(ImageRendering::Pixelated))
    );
  }

  #[test]
  fn sniff_jpeg() {
    let body = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01";
    assert_eq!(
      Media::from_body_head(body),
      Some(Media::Image(ImageRendering::Pixelated))
    );
  }

  #[test]
  fn sniff_gif() {
    assert_eq!(
      Media::from_body_head(b"GIF89a\x10\x00\x10\x00"),
      Some(Media::Image(ImageRendering::Pixelated))
    );
    assert_eq!(
      Media::from_body_head(b"GIF87a\x10\x00\x10\x00"),
      Some(Media::Image(ImageRendering::Pixelated))
    );
  }

  #[test]
  fn sniff_webp() {
    let mut body = b"RIFF".to_vec();
    body.extend_from_slice(&[0, 0, 0, 0]);
    body.extend_from_slice(b"WEBPVP8 ");
    assert_eq!(
      Media::from_body_head(&body),
      Some(Media::Image(ImageRendering::Pixelated))
    );
  }

  #[test]
  fn sniff_mp4() {
    let mut body = vec![0, 0, 0, 0x20];
    body.extend_from_slice(b"ftypisom");
    body.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(Media::from_body_head(&body), Some(Media::Video));
  }

  #[test]
  fn sniff_mp4_mp42_brand() {
    let mut body = vec![0, 0, 0, 0x20];
    body.extend_from_slice(b"ftypmp42");
    body.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(Media::from_body_head(&body), Some(Media::Video));
  }

  #[test]
  fn sniff_mp4_rejects_heic() {
    let mut body = vec![0, 0, 0, 0x20];
    body.extend_from_slice(b"ftypheic");
    body.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(Media::from_body_head(&body), None);
  }

  #[test]
  fn sniff_mp4_rejects_quicktime() {
    let mut body = vec![0, 0, 0, 0x20];
    body.extend_from_slice(b"ftypqt  ");
    body.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(Media::from_body_head(&body), None);
  }

  #[test]
  fn sniff_ogg() {
    let body = b"OggS\x00\x02\x00\x00\x00\x00\x00\x00";
    assert_eq!(Media::from_body_head(body), Some(Media::Audio));
  }

  #[test]
  fn sniff_wav() {
    let mut body = b"RIFF".to_vec();
    body.extend_from_slice(&[0, 0, 0, 0]);
    body.extend_from_slice(b"WAVEfmt ");
    assert_eq!(Media::from_body_head(&body), Some(Media::Audio));
  }

  #[test]
  fn sniff_flac() {
    let body = b"fLaC\x00\x00\x00\x22";
    assert_eq!(Media::from_body_head(body), Some(Media::Audio));
  }

  #[test]
  fn sniff_riff_avi_is_unknown() {
    let mut body = b"RIFF".to_vec();
    body.extend_from_slice(&[0, 0, 0, 0]);
    body.extend_from_slice(b"AVI LIST");
    assert_eq!(Media::from_body_head(&body), None);
  }

  #[test]
  fn sniff_pdf() {
    let body = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3";
    assert_eq!(Media::from_body_head(body), Some(Media::Pdf));
  }
}
