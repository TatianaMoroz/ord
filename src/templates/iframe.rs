use super::*;

pub(crate) struct Iframe {
  inscription_id: InscriptionId,
  kind: IframeKind,
  media: Option<Media>,
}

enum IframeKind {
  Item { i: usize, id: InscriptionId },
  Main,
  Thumbnail,
}

impl Iframe {
  pub(crate) fn item(
    inscription_id: InscriptionId,
    i: usize,
    id: InscriptionId,
    media: Option<Media>,
  ) -> Trusted<Self> {
    Trusted(Self {
      inscription_id,
      kind: IframeKind::Item { i, id },
      media,
    })
  }

  pub(crate) fn main(inscription_id: InscriptionId) -> Trusted<Self> {
    Trusted(Self {
      inscription_id,
      kind: IframeKind::Main,
      media: None,
    })
  }

  pub(crate) fn thumbnail(inscription_id: InscriptionId, media: Option<Media>) -> Trusted<Self> {
    Trusted(Self {
      inscription_id,
      kind: IframeKind::Thumbnail,
      media,
    })
  }

  // Images render as plain <img> tags pointing straight at /content: the
  // browser caches and persists them across scrolling, unlike lazy iframes
  // which are discarded offscreen and reload on re-entry. Arbitrary HTML
  // inscriptions ship script-less: thumbnails are inert (pointer-events:
  // none) yet a grid of live apps accumulates enough CPU that iOS kills the
  // renderer. Recursive art is script-driven and renders blank while inert,
  // so `data-scriptable` marks these for index.js, which grants
  // sandbox=allow-scripts to a capped number of on-screen thumbnails and
  // revokes it as they scroll away. The inert form is the no-JS floor: SVG
  // still paints without scripts. Our own preview wrappers (text, code,
  // markdown, …) need their scripts to render and stay trusted.
  fn thumbnail_body(&self, content_id: InscriptionId, f: &mut Formatter) -> fmt::Result {
    match self.media {
      Some(Media::Image(rendering)) => write!(
        f,
        "<img loading=lazy decoding=async class=rendering-{rendering} src=/content/{content_id}>",
      ),
      Some(Media::Iframe) => write!(
        f,
        "<iframe data-scriptable sandbox scrolling=no loading=lazy src=/preview/{content_id}?thumb=1></iframe>",
      ),
      _ => write!(
        f,
        "<iframe sandbox=allow-scripts scrolling=no loading=lazy src=/preview/{content_id}?thumb=1></iframe>",
      ),
    }
  }
}

impl Display for Iframe {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self.kind {
      IframeKind::Item { i, id } => {
        write!(f, "<a href=/gallery/{}/{i}>", self.inscription_id)?;
        self.thumbnail_body(id, f)?;
        write!(f, "</a>")
      }
      IframeKind::Main => {
        write!(
          f,
          "<iframe sandbox=allow-scripts loading=lazy src=/preview/{}></iframe>",
          self.inscription_id,
        )
      }
      IframeKind::Thumbnail => {
        write!(f, "<a href=/inscription/{}>", self.inscription_id)?;
        self.thumbnail_body(self.inscription_id, f)?;
        write!(f, "</a>")
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gallery_item() {
    assert_regex_match!(
      Iframe::item(inscription_id(1), 2, inscription_id(3), None)
        .0
        .to_string(),
      "<a href=/gallery/1{64}i1/2><iframe sandbox=allow-scripts scrolling=no loading=lazy src=/preview/3{64}i3\\?thumb=1></iframe></a>",
    );
  }

  #[test]
  fn main() {
    assert_regex_match!(
      Iframe::main(inscription_id(1)).0.to_string(),
      "<iframe sandbox=allow-scripts loading=lazy src=/preview/1{64}i1></iframe>",
    );
  }

  #[test]
  fn thumbnail() {
    assert_regex_match!(
      Iframe::thumbnail(inscription_id(1), None).0.to_string(),
      "<a href=/inscription/1{64}i1><iframe sandbox=allow-scripts scrolling=no loading=lazy src=/preview/1{64}i1\\?thumb=1></iframe></a>",
    );
  }

  #[test]
  fn thumbnail_image_renders_img() {
    assert_regex_match!(
      Iframe::thumbnail(
        inscription_id(1),
        Some(Media::Image(media::ImageRendering::Pixelated))
      )
      .0
      .to_string(),
      "<a href=/inscription/1{64}i1><img loading=lazy decoding=async class=rendering-pixelated src=/content/1{64}i1></a>",
    );
  }

  #[test]
  fn thumbnail_html_renders_scriptless_iframe() {
    assert_regex_match!(
      Iframe::thumbnail(inscription_id(1), Some(Media::Iframe))
        .0
        .to_string(),
      "<a href=/inscription/1{64}i1><iframe data-scriptable sandbox scrolling=no loading=lazy src=/preview/1{64}i1\\?thumb=1></iframe></a>",
    );
  }

  #[test]
  fn item_image_renders_img() {
    assert_regex_match!(
      Iframe::item(
        inscription_id(1),
        2,
        inscription_id(3),
        Some(Media::Image(media::ImageRendering::Auto))
      )
      .0
      .to_string(),
      "<a href=/gallery/1{64}i1/2><img loading=lazy decoding=async class=rendering-auto src=/content/3{64}i3></a>",
    );
  }
}
