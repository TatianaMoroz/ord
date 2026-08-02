use super::*;

#[derive(Boilerplate)]
pub(crate) struct ComingSoonHtml {}

impl PageContent for ComingSoonHtml {
  fn title(&self) -> String {
    "Coming Soon".into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn coming_soon() {
    assert_regex_match!(ComingSoonHtml {}, "<h1>Coming Soon\\.</h1>\n",);
  }
}
