use super::*;

#[derive(Boilerplate)]
pub struct ItemHtml {
  pub gallery_id: InscriptionId,
  pub gallery_number: i32,
  pub gallery_title: Option<String>,
  pub i: usize,
  pub item: Item,
  pub total: usize,
}

impl PageContent for ItemHtml {
  fn title(&self) -> String {
    let gallery_part = self
      .gallery_title
      .clone()
      .unwrap_or_else(|| format!("Gallery {}", self.gallery_number));
    let item_part = self
      .item
      .attributes
      .title
      .clone()
      .unwrap_or_else(|| format!("Item {}", self.i));
    format!("{} / {}", gallery_part, item_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn body() {
    assert_regex_match!(
      ItemHtml {
        gallery_id: inscription_id(2),
        gallery_number: 1,
        gallery_title: Some("Bar".into()),
        i: 2,
        total: 5,
        item: Item {
          id: Some(inscription_id(1)),
          attributes: Attributes {
            title: Some("foo".into()),
            traits: Traits::default(),
          },
          index: None,
        },
      },
      "
        <h1>foo</h1>
        <div class=subtitle-row>
          <p class=subtitle><a href=/inscription/2{64}i2>\\s*Bar\\s*</a> / Item 2</p>
          <div class=title-links data-ord-path=/inscription/1{64}i1></div>
        </div>
        <div class=\"inscription gallery-item-nav\">
        <a class=prev href=/gallery/2{64}i2/1>❮</a>
        <iframe .* src=/preview/1{64}i1></iframe>
        <a class=next href=/gallery/2{64}i2/3>❯</a>
        </div>
        <dl>
          <dt>inscription</dt>
          <dd><a class=collapse href=/inscription/1{64}i1>1{64}i1</a></dd>
          <dt>gallery</dt>
          <dd><a class=collapse href=/inscription/2{64}i2>2{64}i2</a></dd>
          <dt>title</dt>
        <dd>foo</dd>

        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn body_without_item_title() {
    assert_regex_match!(
      ItemHtml {
        gallery_id: inscription_id(2),
        gallery_number: 1,
        gallery_title: Some("Bar".into()),
        i: 2,
        total: 5,
        item: Item {
          id: Some(inscription_id(1)),
          attributes: Attributes::default(),
          index: None,
        },
      },
      "
        <h1>Gallery 1 Item 2</h1>
        <div class=subtitle-row>
          <div class=title-links data-ord-path=/inscription/1{64}i1></div>
        </div>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn title() {
    assert_eq!(
      ItemHtml {
        gallery_id: inscription_id(2),
        gallery_number: 1,
        gallery_title: Some("Bar".into()),
        i: 2,
        total: 5,
        item: Item {
          id: Some(inscription_id(1)),
          attributes: Attributes {
            title: Some("foo".into()),
            traits: Traits::default(),
          },
          index: None,
        },
      }
      .title(),
      "Bar / foo",
    );
  }
}
