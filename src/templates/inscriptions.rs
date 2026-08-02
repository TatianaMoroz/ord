use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Default, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
  #[default]
  Newest,
  Oldest,
}

impl std::fmt::Display for Sort {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.write_str(match self {
      Sort::Newest => "newest",
      Sort::Oldest => "oldest",
    })
  }
}

#[derive(Boilerplate)]
pub(crate) struct InscriptionsHtml {
  pub(crate) inscriptions: Vec<(InscriptionId, Option<Media>)>,
  pub(crate) prev: Option<u32>,
  pub(crate) next: Option<u32>,
  pub(crate) sort: Sort,
  pub(crate) cursed: bool,
}

impl InscriptionsHtml {
  pub(crate) fn query_string(&self) -> &'static str {
    match (self.sort, self.cursed) {
      (Sort::Newest, false) => "",
      (Sort::Oldest, false) => "?sort=oldest",
      (Sort::Newest, true) => "?cursed=1",
      (Sort::Oldest, true) => "?sort=oldest&cursed=1",
    }
  }

  pub(crate) fn cursed_checked(&self) -> &'static str {
    if self.cursed { " checked" } else { "" }
  }

  pub(crate) fn selected_if(&self, sort: Sort) -> &'static str {
    if self.sort == sort { " selected" } else { "" }
  }
}

impl PageContent for InscriptionsHtml {
  fn title(&self) -> String {
    "Inscriptions".into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn without_prev_and_next() {
    assert_regex_match!(
      InscriptionsHtml {
        inscriptions: vec![(inscription_id(1), None), (inscription_id(2), None)],
        prev: None,
        next: None,
        sort: Sort::Newest,
        cursed: false,
      },
      "
        .*<h1>All Inscriptions</h1>.*
        <div class=thumbnails>
          <a href=/inscription/1{64}i1><iframe .* src=/preview/1{64}i1\\?thumb=1></iframe></a>
          <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2\\?thumb=1></iframe></a>
        </div>
        .*
        prev
        next
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn with_prev_and_next() {
    assert_regex_match!(
      InscriptionsHtml {
        inscriptions: vec![(inscription_id(1), None), (inscription_id(2), None)],
        prev: Some(1),
        next: Some(2),
        sort: Sort::Newest,
        cursed: false,
      },
      "
        .*<a class=prev href=/inscriptions/1>prev</a>
        <a class=next href=/inscriptions/2>next</a>.*
      "
      .unindent()
    );
  }

  #[test]
  fn oldest_sort_preserved_in_pagination_links() {
    assert_regex_match!(
      InscriptionsHtml {
        inscriptions: vec![(inscription_id(1), None)],
        prev: Some(0),
        next: Some(2),
        sort: Sort::Oldest,
        cursed: false,
      },
      "
        .*<a class=prev href=/inscriptions/0\\?sort=oldest>prev</a>
        <a class=next href=/inscriptions/2\\?sort=oldest>next</a>.*
      "
      .unindent()
    );
  }
}
