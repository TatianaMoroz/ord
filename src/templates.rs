use {super::*, boilerplate::Boilerplate};

pub(crate) use {
  crate::subcommand::server::ServerConfig,
  address::AddressHtml,
  attributes::AttributesHtml,
  block::BlockHtml,
  children::ChildrenHtml,
  clock::ClockSvg,
  collections::CollectionsHtml,
  coming_soon::ComingSoonHtml,
  embed::{EmbedAudioHtml, EmbedImageHtml, EmbedUnknownHtml, EmbedVideoHtml},
  galleries::GalleriesHtml,
  gallery::GalleryHtml,
  home::HomeHtml,
  iframe::Iframe,
  input::InputHtml,
  inscriptions::{InscriptionsHtml, Sort as InscriptionsSort},
  inscriptions_block::InscriptionsBlockHtml,
  metadata::MetadataHtml,
  output::OutputHtml,
  parents::ParentsHtml,
  preview::{
    PreviewAudioHtml, PreviewCodeHtml, PreviewFontHtml, PreviewImageHtml, PreviewMarkdownHtml,
    PreviewModelHtml, PreviewPdfHtml, PreviewTextHtml, PreviewUnknownHtml, PreviewVideoHtml,
  },
  rare::RareTxt,
  rune_not_found::RuneNotFoundHtml,
  sat::SatHtml,
  satscard::SatscardHtml,
};

pub use {
  blocks::BlocksHtml,
  inscription::{text_title, Crumb, InscriptionHtml, SatInscription},
  item::ItemHtml,
  rune::RuneHtml,
  runes::RunesHtml,
  status::StatusHtml,
  transaction::TransactionHtml,
};

pub mod address;
mod attributes;
pub mod block;
pub mod blocks;
mod children;
mod clock;
pub mod collections;
mod coming_soon;
mod embed;
mod galleries;
mod gallery;
mod home;
mod iframe;
mod input;
pub mod inscription;
pub mod inscriptions;
mod inscriptions_block;
mod item;
mod metadata;
pub mod output;
mod parents;
mod preview;
mod rare;
pub mod rune;
pub mod rune_not_found;
pub mod runes;
pub mod sat;
mod satscard;
pub mod status;
pub mod transaction;

#[derive(Boilerplate)]
pub struct PageHtml<T: PageContent> {
  content: T,
  config: Arc<ServerConfig>,
}

impl<T> PageHtml<T>
where
  T: PageContent,
{
  pub fn new(content: T, config: Arc<ServerConfig>) -> Self {
    Self { content, config }
  }

  fn og_image(&self) -> String {
    format!("{}/static/favicon.png", self.page_origin())
  }

  fn page_origin(&self) -> String {
    if let Some(origin) = &self.config.csp_origin {
      origin.clone()
    } else if let Some(domain) = &self.config.domain {
      format!("https://{domain}")
    } else {
      "https://ordinals.com".into()
    }
  }

  fn oembed_link(&self) -> String {
    let Some(path) = self.content.oembed_url() else {
      return String::new();
    };
    let absolute = format!("{}{}", self.page_origin(), path);
    format!(
      r#"<link rel=alternate type='application/json+oembed' href='/oembed?url={absolute}' title='{title}'>"#,
      title = self.content.title(),
    )
  }

  fn home_text(&self) -> &'static str {
    if self.config.chain == Chain::Mainnet {
      "Ordinals"
    } else {
      "Ordinals.Gallery"
    }
  }

  fn superscript(&self) -> String {
    if self.config.chain == Chain::Mainnet {
      "Gallery".into()
    } else {
      self.config.chain.to_string()
    }
  }
}

pub trait PageContent: Display + 'static {
  fn title(&self) -> String;

  fn oembed_url(&self) -> Option<String> {
    None
  }

  fn page(self, server_config: Arc<ServerConfig>) -> PageHtml<Self>
  where
    Self: Sized,
  {
    PageHtml::new(self, server_config)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Foo;

  impl Display for Foo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
      write!(f, "<h1>Foo</h1>")
    }
  }

  impl PageContent for Foo {
    fn title(&self) -> String {
      "Foo".to_string()
    }
  }

  #[test]
  fn page() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Mainnet,
        csp_origin: Some("https://signet.ordinals.com".into()),
        domain: Some("signet.ordinals.com".into()),
        index_sats: true,
        ..default()
      }),),
      r"<!doctype html>
<html lang=en>
  <head>
    <meta charset=utf-8>
    <meta name=format-detection content='telephone=no'>
    <meta name=viewport content='width=device-width,initial-scale=1.0'>
    <meta property=og:title content='Foo'>
    <meta property=og:image content='https://signet.ordinals.com/static/favicon.png'>
    <meta property=twitter:card content=summary>
    <title>Foo</title>
    <link rel=alternate href=/feed.xml type=application/rss\+xml title='Inscription Feed'>
\s*
    <link rel=icon href=/static/favicon.png>
    <link rel=icon href=/static/favicon.svg>
    <script src=/static/theme-init.js></script>
    <link rel=stylesheet href=/static/index.css>
    <link rel=stylesheet href=/static/modern-normalize.css>
    <script src=/static/marketplaces.js></script>
    <script src=/static/index.js></script>
    <script src=/static/inscription-embed.js defer></script>
  </head>
  <body>
  <header>
    <nav>
      <a href=/ title=home>Ordinals<sup>Gallery</sup></a>
      .*
      <a href=/clock title=clock>.*</a>
      <a href=/rare.txt title=rare>.*</a>
      .*
      <form action=/search method=get>
        <input type=text .*>
        <input class=icon type=image .*>
      </form>
    </nav>
  </header>
  <main>
<h1>Foo</h1>
  </main>
  </body>
</html>
"
    );
  }

  #[test]
  fn page_mainnet() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Mainnet,
        csp_origin: None,
        domain: None,
        index_sats: true,
        ..default()
      })),
      r".*<nav>\s*<a href=/ title=home>Ordinals<sup>Gallery</sup></a>.*"
    );
  }

  #[test]
  fn page_no_sat_index() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Mainnet,
        csp_origin: None,
        domain: None,
        index_sats: false,
        ..default()
      })),
      r".*<nav>\s*<a href=/ title=home>Ordinals<sup>Gallery</sup></a>.*<a href=/clock title=clock>.*</a>.*<form action=/search.*",
    );
  }

  #[test]
  fn page_signet() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Signet,
        csp_origin: None,
        domain: None,
        index_sats: true,
        ..default()
      })),
      r".*<nav>\s*<a href=/ title=home>Ordinals\.Gallery<sup>signet</sup></a>.*"
    );
  }

  #[test]
  fn og_image_prefers_csp_origin_over_domain() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Mainnet,
        csp_origin: Some("https://ordinals.gallery".into()),
        domain: Some("some-laptop.local".into()),
        ..default()
      })),
      r".*<meta property=og:image content='https://ordinals\.gallery/static/favicon\.png'>.*"
    );
  }

  #[test]
  fn og_image_falls_back_to_ordinals_com() {
    assert_regex_match!(
      Foo.page(Arc::new(ServerConfig {
        chain: Chain::Mainnet,
        csp_origin: None,
        domain: None,
        ..default()
      })),
      r".*<meta property=og:image content='https://ordinals\.com/static/favicon\.png'>.*"
    );
  }
}
