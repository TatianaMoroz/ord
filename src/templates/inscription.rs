use {super::*, crate::inscriptions::opus_metadata};

#[derive(Clone, Debug)]
pub struct SatInscription {
  pub id: InscriptionId,
  pub label: String,
}

#[derive(Clone, Debug)]
pub struct Crumb {
  pub id: InscriptionId,
  pub title: String,
  pub reinscriptions: Vec<SatInscription>,
  pub children: Vec<SatInscription>,
  pub more_children: bool,
}

/// View helper: the breadcrumb trail set collapsed into a single-line
/// inline form with a vertical fork only where the parent paths diverge.
///
/// * `prefix`  — crumbs shared by every trail at the start (may be empty).
/// * `middles` — per-trail divergent middle (empty when only one trail
///   exists; with multiple trails, one entry per trail in input order; an
///   entry may itself be empty when a trail goes directly from the common
///   prefix into the common suffix).
/// * `suffix`  — crumbs shared by every trail at the end (always contains
///   at least the current inscription's crumb when any trail is present).
#[derive(Clone, Debug)]
pub struct BreadcrumbLayout {
  pub prefix: Vec<Crumb>,
  pub middles: Vec<Vec<Crumb>>,
  pub suffix: Vec<Crumb>,
}

pub fn text_title(inscription: &Inscription) -> Option<String> {
  const MAX_LEN: usize = 64;

  if inscription.content_encoding().is_some() {
    return None;
  }

  if !inscription.content_type()?.starts_with("text/plain") {
    return None;
  }

  let text = std::str::from_utf8(inscription.body()?).ok()?.trim();

  if text.is_empty() || text.contains(['\n', '\r']) || text.chars().count() > MAX_LEN {
    return None;
  }

  Some(text.to_string())
}

#[derive(Boilerplate, Default)]
pub struct InscriptionHtml {
  pub breadcrumbs: Vec<Vec<Crumb>>,
  pub chain: Chain,
  pub charms: u16,
  pub child_count: u64,
  pub children: Vec<(InscriptionId, Option<Media>)>,
  pub fee: u64,
  pub gallery_media: Vec<Option<Media>>,
  pub height: u32,
  pub id: InscriptionId,
  pub inscription: Inscription,
  pub next: Option<InscriptionId>,
  pub number: i32,
  pub output: Option<TxOut>,
  pub parents: Vec<(InscriptionId, Option<Media>)>,
  pub previous: Option<InscriptionId>,
  pub properties: Properties,
  pub rune: Option<SpacedRune>,
  pub sat: Option<Sat>,
  pub satpoint: SatPoint,
  pub timestamp: DateTime<Utc>,
}

impl PageContent for InscriptionHtml {
  fn title(&self) -> String {
    format!("Inscription {}", self.number)
  }

  fn oembed_url(&self) -> Option<String> {
    Some(format!("/inscription/{}", self.id))
  }
}

impl InscriptionHtml {
  /// Collapse `self.breadcrumbs` into a `BreadcrumbLayout`: longest common
  /// prefix + per-trail divergent middle + longest common suffix. With a
  /// single trail (or trails that dedupe to one) `middles` is empty and
  /// the template renders a single line; with multiple distinct trails it
  /// renders a vertical fork only across the divergent middle.
  pub fn breadcrumb_layout(&self) -> BreadcrumbLayout {
    // Two trails through equivalent ancestors shouldn't produce a fork of
    // identical rows, so dedupe by the trail's id sequence first.
    let mut trails: Vec<Vec<Crumb>> = Vec::new();
    for trail in &self.breadcrumbs {
      if !trails.iter().any(|existing| {
        existing.len() == trail.len()
          && existing.iter().zip(trail).all(|(a, b)| a.id == b.id)
      }) {
        trails.push(trail.clone());
      }
    }

    if trails.is_empty() {
      return BreadcrumbLayout {
        prefix: Vec::new(),
        middles: Vec::new(),
        suffix: Vec::new(),
      };
    }

    if trails.len() == 1 {
      let mut single = trails.into_iter().next().unwrap();
      let suffix = match single.pop() {
        Some(current) => vec![current],
        None => Vec::new(),
      };
      return BreadcrumbLayout {
        prefix: single,
        middles: Vec::new(),
        suffix,
      };
    }

    // Multi-trail: longest common prefix, capped so the current crumb is
    // always reserved for the suffix.
    let min_len = trails.iter().map(|t| t.len()).min().unwrap_or(0);
    let max_prefix = min_len.saturating_sub(1);
    let mut prefix_len = 0;
    while prefix_len < max_prefix
      && trails
        .iter()
        .all(|t| t[prefix_len].id == trails[0][prefix_len].id)
    {
      prefix_len += 1;
    }

    // Longest common suffix, walked from each trail's end, capped at the
    // remaining length of the shortest trail so middles never go negative.
    let max_suffix = trails
      .iter()
      .map(|t| t.len() - prefix_len)
      .min()
      .unwrap_or(0);
    let mut suffix_len = 0;
    while suffix_len < max_suffix
      && trails.iter().all(|t| {
        t[t.len() - 1 - suffix_len].id
          == trails[0][trails[0].len() - 1 - suffix_len].id
      })
    {
      suffix_len += 1;
    }

    let prefix = trails[0][..prefix_len].to_vec();
    let suffix = trails[0][trails[0].len() - suffix_len..].to_vec();
    let middles: Vec<Vec<Crumb>> = trails
      .iter()
      .map(|t| t[prefix_len..t.len() - suffix_len].to_vec())
      .collect();

    BreadcrumbLayout {
      prefix,
      middles,
      suffix,
    }
  }

  fn metadata_sections_from_values(
    opus_metadata: Option<Value>,
    metadata: Option<Value>,
  ) -> Vec<(&'static str, Value)> {
    let mut sections = Vec::new();

    if let Some(opus_metadata) = opus_metadata {
      sections.push(("opus metadata", opus_metadata));
    }

    if let Some(metadata) = metadata {
      sections.push(("metadata", metadata));
    }

    sections
  }

  pub fn metadata_sections(&self) -> Vec<(&'static str, Value)> {
    Self::metadata_sections_from_values(self.opus_metadata(), self.inscription.metadata())
  }

  pub fn opus_metadata(&self) -> Option<Value> {
    if self.inscription.content_encoding().is_some() {
      return None;
    }

    let content_type = self.inscription.content_type()?;
    if !opus_metadata::is_opus_content_type(content_type) {
      return None;
    }

    opus_metadata::structured(self.inscription.body()?)
  }

  pub fn burn_metadata(&self) -> Option<Value> {
    let script_pubkey = &self.output.as_ref()?.script_pubkey;

    if !script_pubkey.is_op_return() {
      return None;
    }

    let script::Instruction::PushBytes(metadata) = script_pubkey.instructions().nth(1)?.ok()?
    else {
      return None;
    };

    ciborium::from_reader(Cursor::new(metadata)).ok()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn without_sat_nav_links_or_output() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <div>❮</div>
        <iframe .* src=/preview/1{64}i1></iframe>
        <div>❯</div>
        </div>
        <dl>
          <dt>id</dt>
          <dd class=collapse>1{64}i1</dd>
          <dt>preview</dt>
          <dd><a href=/preview/1{64}i1>link</a></dd>
          <dt>embed</dt>
          <dd>
            <a href=/embed/1{64}i1>link</a>
            <button type=button data-embed-copy data-inscription-id=1{64}i1>copy embed code</button>
          </dd>
          <dt>content</dt>
          <dd><a href=/content/1{64}i1>link</a></dd>
          <dt>content length</dt>
          <dd>10 bytes</dd>
          <dt>content type</dt>
          <dd>text/plain;charset=utf-8</dd>
          <dt>timestamp</dt>
          <dd><time>1970-01-01 00:00:00 UTC</time></dd>
          <dt>height</dt>
          <dd><a href=/block/0>0</a></dd>
          <dt>fee</dt>
          <dd>1</dd>
          <dt>reveal transaction</dt>
          <dd><a class=collapse href=/tx/1{64}>1{64}</a></dd>
          <dt>location</dt>
          <dd><a class=collapse href=/satpoint/1{64}:1:0>1{64}:1:0</a></dd>
          <dt>output</dt>
          <dd><a class=collapse href=/output/1{64}:1>1{64}:1</a></dd>
          <dt>offset</dt>
          <dd>0</dd>
          <dt>ethereum teleburn address</dt>
          <dd class=collapse>0xa1DfBd1C519B9323FD7Fd8e498Ac16c2E502F059</dd>
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_title() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        properties: Properties {
          attributes: Attributes {
            title: Some("Bitcoin Shrooms".into()),
            ..default()
          },
          ..default()
        },
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>Bitcoin Shrooms</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn bitmap_text_used_as_heading() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "12345.bitmap"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>12345.bitmap</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn long_text_keeps_number_heading() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription(
          "text/plain;charset=utf-8",
          "this text is far too long to be used as an inscription heading so it is ignored",
        ),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>Inscription 1</h1>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn with_breadcrumbs() {
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![vec![
          Crumb {
            id: inscription_id(2),
            title: "MoBA".into(),
            reinscriptions: Vec::new(),
            children: Vec::new(),
            more_children: false,
          },
          Crumb {
            id: inscription_id(1),
            title: "Bitcoin Shrooms".into(),
            reinscriptions: Vec::new(),
            children: Vec::new(),
            more_children: false,
          },
        ]],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <div class=breadcrumbs>
        .*
        <a href=/inscription/2{64}i2>MoBA</a>
        .*
        <span class=current>Bitcoin Shrooms</span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn breadcrumb_reinscriptions_render_dropdown() {
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![vec![
          Crumb {
            id: inscription_id(2),
            title: "MoBA".into(),
            reinscriptions: vec![
              SatInscription {
                id: inscription_id(2),
                label: "#1".into(),
              },
              SatInscription {
                id: inscription_id(3),
                label: "12345.bitmap".into(),
              },
            ],
            children: Vec::new(),
            more_children: false,
          },
          Crumb {
            id: inscription_id(1),
            title: "Bitcoin Shrooms".into(),
            reinscriptions: Vec::new(),
            children: Vec::new(),
            more_children: false,
          },
        ]],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <a href=/inscription/2{64}i2>MoBA</a>
        <button class=crumb-toggle type=button aria-label=reinscriptions>.*</button>
        <span class=crumb-menu>
        <a href=/inscription/2{64}i2>#1</a>
        <a href=/inscription/3{64}i3>12345.bitmap</a>
        </span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn breadcrumb_children_render_dropdown() {
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![vec![
          Crumb {
            id: inscription_id(2),
            title: "MoBA".into(),
            reinscriptions: Vec::new(),
            children: vec![
              SatInscription {
                id: inscription_id(4),
                label: "Sound Gallery".into(),
              },
              SatInscription {
                id: inscription_id(5),
                label: "Image Gallery".into(),
              },
            ],
            more_children: false,
          },
          Crumb {
            id: inscription_id(1),
            title: "Bitcoin Shrooms".into(),
            reinscriptions: Vec::new(),
            children: Vec::new(),
            more_children: false,
          },
        ]],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <a href=/inscription/2{64}i2>MoBA</a>
        <button class=crumb-toggle type=button aria-label=reinscriptions>.*</button>
        <span class=crumb-menu>
        <a href=/inscription/4{64}i4>Sound Gallery</a>
        <a href=/inscription/5{64}i5>Image Gallery</a>
        </span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn breadcrumb_fork_collapses_common_prefix_and_suffix() {
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![
          vec![
            Crumb {
              id: inscription_id(2),
              title: "MoBA".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(3),
              title: "Inscription Clubs".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(1),
              title: "Ordinal Archaeology".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
          ],
          vec![
            Crumb {
              id: inscription_id(2),
              title: "MoBA".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(4),
              title: "Library".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(1),
              title: "Ordinal Archaeology".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
          ],
        ],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <a href=/inscription/2{64}i2>MoBA</a>
        .*
        <div class=breadcrumb-fork>
        <div class=breadcrumb-fork-row>
        .*<a href=/inscription/3{64}i3>Inscription Clubs</a>.*
        </div>
        <div class=breadcrumb-fork-row>
        .*<a href=/inscription/4{64}i4>Library</a>.*
        </div>
        </div>
        .*
        <span class=current>Ordinal Archaeology</span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn breadcrumb_fork_blank_row_when_trail_has_no_divergent_middle() {
    // Trail A: [X, Y, Z, current]. Trail B: [X, Z, current]. After
    // collapsing the shared prefix [X] and the shared suffix [Z, current],
    // A's middle is [Y] and B's middle is empty — render B's row blank.
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![
          vec![
            Crumb {
              id: inscription_id(2),
              title: "X".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(3),
              title: "Y".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(4),
              title: "Z".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(1),
              title: "Current".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
          ],
          vec![
            Crumb {
              id: inscription_id(2),
              title: "X".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(4),
              title: "Z".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
            Crumb {
              id: inscription_id(1),
              title: "Current".into(),
              reinscriptions: Vec::new(),
              children: Vec::new(),
              more_children: false,
            },
          ],
        ],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <a href=/inscription/2{64}i2>X</a>
        .*
        <div class=breadcrumb-fork>
        <div class=breadcrumb-fork-row>
        .*<a href=/inscription/3{64}i3>Y</a>.*
        </div>
        <div class=breadcrumb-fork-row>
        </div>
        </div>
        .*
        <a href=/inscription/4{64}i4>Z</a>
        .*
        <span class=current>Current</span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn breadcrumb_combines_reinscriptions_and_children_with_more() {
    assert_regex_match!(
      InscriptionHtml {
        breadcrumbs: vec![vec![
          Crumb {
            id: inscription_id(2),
            title: "MoBA".into(),
            reinscriptions: vec![
              SatInscription {
                id: inscription_id(3),
                label: "12345.bitmap".into(),
              },
            ],
            children: vec![
              SatInscription {
                id: inscription_id(4),
                label: "Sound Gallery".into(),
              },
            ],
            more_children: true,
          },
          Crumb {
            id: inscription_id(1),
            title: "Bitcoin Shrooms".into(),
            reinscriptions: Vec::new(),
            children: Vec::new(),
            more_children: false,
          },
        ]],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*
        <a href=/inscription/2{64}i2>MoBA</a>
        <button class=crumb-toggle type=button aria-label=reinscriptions>.*</button>
        <span class=crumb-menu>
        <a href=/inscription/3{64}i3>12345.bitmap</a>
        <div class=crumb-menu-divider></div>
        <a href=/inscription/4{64}i4>Sound Gallery</a>
        <a href=/children/2{64}i2>all children</a>
        </span>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn with_output() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        output: Some(tx_out(1, address(0))),
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        .*<h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <div>❮</div>
        <iframe .* src=/preview/1{64}i1></iframe>
        <div>❯</div>
        </div>
        <dl>
          .*
          <dt>address</dt>
          <dd><a class=collapse href=/address/bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4>bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4</a></dd>
          <dt>value</dt>
          <dd>1</dd>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_sat() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        output: Some(tx_out(1, address(0))),
        sat: Some(Sat(1)),
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        .*
        <dl>
          .*
          <dt>sat</dt>
          <dd><a href=/sat/1>1</a></dd>
          <dt>sat name</dt>
          <dd><a href=/sat/nvtdijuwxlo>nvtdijuwxlo</a></dd>
          <dt>preview</dt>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_prev_and_next() {
    assert_regex_match!(
      InscriptionHtml {
        children: Vec::new(),
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(2),
        next: Some(inscription_id(3)),
        number: 1,
        output: Some(tx_out(1, address(0))),
        previous: Some(inscription_id(1)),
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <a class=prev href=/inscription/1{64}i1>❮</a>
        <iframe .* src=/preview/2{64}i2></iframe>
        <a class=next href=/inscription/3{64}i3>❯</a>
        </div>
        .*
      "
      .unindent()
    );
  }

  #[test]
  fn with_cursed_and_unbound() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(2),
        number: -1,
        output: Some(tx_out(1, address(0))),
        satpoint: SatPoint {
          outpoint: unbound_outpoint(),
          offset: 0
        },
        timestamp: timestamp(0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        .*
        <dl>
          .*
          <dt>location</dt>
          <dd><a class=collapse href=/satpoint/0{64}:0:0>0{64}:0:0</a></dd>
          <dt>output</dt>
          <dd><a class=collapse href=/output/0{64}:0>0{64}:0</a></dd>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_parent() {
    assert_regex_match!(
      InscriptionHtml {
        parents: vec![(inscription_id(2), None)],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <div>❮</div>
        <iframe .* src=/preview/1{64}i1></iframe>
        <div>❯</div>
        </div>
        <dl>
          <dt>parents</dt>
          <dd>
            <div class=thumbnails>
              <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2\\?thumb=1></iframe></a>
            </div>
            <div class=center>
              <a href=/parents/1{64}i1>all</a>
            </div>
          </dd>
          <dt>id</dt>
          <dd class=collapse>1{64}i1</dd>
          <dt>preview</dt>
          <dd><a href=/preview/1{64}i1>link</a></dd>
          <dt>embed</dt>
          <dd>
            <a href=/embed/1{64}i1>link</a>
            <button type=button data-embed-copy data-inscription-id=1{64}i1>copy embed code</button>
          </dd>
          <dt>content</dt>
          <dd><a href=/content/1{64}i1>link</a></dd>
          <dt>content length</dt>
          <dd>10 bytes</dd>
          <dt>content type</dt>
          <dd>text/plain;charset=utf-8</dd>
          <dt>timestamp</dt>
          <dd><time>1970-01-01 00:00:00 UTC</time></dd>
          <dt>height</dt>
          <dd><a href=/block/0>0</a></dd>
          <dt>fee</dt>
          <dd>1</dd>
          <dt>reveal transaction</dt>
          <dd><a class=collapse href=/tx/1{64}>1{64}</a></dd>
          <dt>location</dt>
          <dd><a class=collapse href=/satpoint/1{64}:1:0>1{64}:1:0</a></dd>
          <dt>output</dt>
          <dd><a class=collapse href=/output/1{64}:1>1{64}:1</a></dd>
          <dt>offset</dt>
          <dd>0</dd>
          <dt>ethereum teleburn address</dt>
          <dd class=collapse>0xa1DfBd1C519B9323FD7Fd8e498Ac16c2E502F059</dd>
        </dl>
"
      .unindent()
    );
  }

  #[test]
  fn with_children() {
    assert_regex_match!(
      InscriptionHtml {
        child_count: 2,
        children: vec![(inscription_id(2), None), (inscription_id(3), None)],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <div>❮</div>
        <iframe .* src=/preview/1{64}i1></iframe>
        <div>❯</div>
        </div>
        <dl>
          <dt class=with-toolbar>children
            <div class=gallery-toolbar>
              <button class=\"gallery-view-btn active\" type=button data-mode=scroll aria-label=\"strip view\"><img class=icon src=/static/view-strip.svg></button>
              <button class=gallery-view-btn type=button data-mode=all aria-label=\"grid view\"><img class=icon src=/static/view-grid.svg></button>
            </div>
          </dt>
          <dd>
            <div class=\"gallery-row gallery-mode-scroll\" data-gallery-total=\"2\" data-load-more-url=\"/r/children/1{64}i1\">
              <button class=gallery-prev type=button aria-label=\"previous\">❮</button>
              <div class=thumbnails>
                <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2\\?thumb=1></iframe></a>
                <a href=/inscription/3{64}i3><iframe .* src=/preview/3{64}i3\\?thumb=1></iframe></a>
              </div>
              <button class=gallery-next type=button aria-label=\"next\">❯</button>
            </div>
            <div class=center>
              <a href=/children/1{64}i1>all \\(2\\)</a>
            </div>
          </dd>
          <dt>id</dt>
          <dd class=collapse>1{64}i1</dd>
          <dt>preview</dt>
          <dd><a href=/preview/1{64}i1>link</a></dd>
          <dt>embed</dt>
          <dd>
            <a href=/embed/1{64}i1>link</a>
            <button type=button data-embed-copy data-inscription-id=1{64}i1>copy embed code</button>
          </dd>
          <dt>content</dt>
          <dd><a href=/content/1{64}i1>link</a></dd>
          <dt>content length</dt>
          <dd>10 bytes</dd>
          <dt>content type</dt>
          <dd>text/plain;charset=utf-8</dd>
          <dt>timestamp</dt>
          <dd><time>1970-01-01 00:00:00 UTC</time></dd>
          <dt>height</dt>
          <dd><a href=/block/0>0</a></dd>
          <dt>fee</dt>
          <dd>1</dd>
          <dt>reveal transaction</dt>
          <dd><a class=collapse href=/tx/1{64}>1{64}</a></dd>
          <dt>location</dt>
          <dd><a class=collapse href=/satpoint/1{64}:1:0>1{64}:1:0</a></dd>
          <dt>output</dt>
          <dd><a class=collapse href=/output/1{64}:1>1{64}:1</a></dd>
          <dt>offset</dt>
          <dd>0</dd>
          <dt>ethereum teleburn address</dt>
          <dd class=collapse>0xa1DfBd1C519B9323FD7Fd8e498Ac16c2E502F059</dd>
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_paginated_children() {
    assert_regex_match!(
      InscriptionHtml {
        child_count: 1,
        children: vec![(inscription_id(2), None)],
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        <div class=subtitle-row>
          <p class=subtitle>Inscription 1</p>
          <div class=title-links></div>
        </div>
        <div class=inscription>
        <div>❮</div>
        <iframe .* src=/preview/1{64}i1></iframe>
        <div>❯</div>
        </div>
        <dl>
          <dt class=with-toolbar>children
            <div class=gallery-toolbar>
              <button class=\"gallery-view-btn active\" type=button data-mode=scroll aria-label=\"strip view\"><img class=icon src=/static/view-strip.svg></button>
              <button class=gallery-view-btn type=button data-mode=all aria-label=\"grid view\"><img class=icon src=/static/view-grid.svg></button>
            </div>
          </dt>
          <dd>
            <div class=\"gallery-row gallery-mode-scroll\" data-gallery-total=\"1\" data-load-more-url=\"/r/children/1{64}i1\">
              <button class=gallery-prev type=button aria-label=\"previous\">❮</button>
              <div class=thumbnails>
                <a href=/inscription/2{64}i2><iframe .* src=/preview/2{64}i2\\?thumb=1></iframe></a>
              </div>
              <button class=gallery-next type=button aria-label=\"next\">❯</button>
            </div>
            <div class=center>
              <a href=/children/1{64}i1>all \\(1\\)</a>
            </div>
          </dd>
          <dt>id</dt>
          <dd class=collapse>1{64}i1</dd>
          <dt>preview</dt>
          <dd><a href=/preview/1{64}i1>link</a></dd>
          <dt>embed</dt>
          <dd>
            <a href=/embed/1{64}i1>link</a>
            <button type=button data-embed-copy data-inscription-id=1{64}i1>copy embed code</button>
          </dd>
          <dt>content</dt>
          <dd><a href=/content/1{64}i1>link</a></dd>
          <dt>content length</dt>
          <dd>10 bytes</dd>
          <dt>content type</dt>
          <dd>text/plain;charset=utf-8</dd>
          <dt>timestamp</dt>
          <dd><time>1970-01-01 00:00:00 UTC</time></dd>
          <dt>height</dt>
          <dd><a href=/block/0>0</a></dd>
          <dt>fee</dt>
          <dd>1</dd>
          <dt>reveal transaction</dt>
          <dd><a class=collapse href=/tx/1{64}>1{64}</a></dd>
          <dt>location</dt>
          <dd><a class=collapse href=/satpoint/1{64}:1:0>1{64}:1:0</a></dd>
          <dt>output</dt>
          <dd><a class=collapse href=/output/1{64}:1>1{64}:1</a></dd>
          <dt>offset</dt>
          <dd>0</dd>
          <dt>ethereum teleburn address</dt>
          <dd class=collapse>0xa1DfBd1C519B9323FD7Fd8e498Ac16c2E502F059</dd>
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_rune() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: inscription("text/plain;charset=utf-8", "HELLOWORLD"),
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        rune: Some(SpacedRune {
          rune: Rune(26),
          spacers: 1
        }),
        ..default()
      },
      "
        <h1>HELLOWORLD</h1>
        .*
        <dl>
          <dt>rune</dt>
          <dd><a href=/rune/A•A>A•A</a></dd>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_content_encoding() {
    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: Inscription {
          content_encoding: Some(BROTLI.into()),
          ..inscription("text/plain;charset=utf-8", "HELLOWORLD")
        },
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        ..default()
      },
      "
        <h1>Inscription 1</h1>
        .*
        <dl>
          .*
          <dt>content encoding</dt>
          <dd>br</dd>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn with_burn_metadata() {
    let script_pubkey = script::Builder::new()
      .push_opcode(opcodes::all::OP_RETURN)
      .push_slice([
        0xA2, 0x63, b'f', b'o', b'o', 0x63, b'b', b'a', b'r', 0x63, b'b', b'a', b'z', 0x01,
      ])
      .into_script();

    assert_regex_match!(
      InscriptionHtml {
        fee: 1,
        inscription: Inscription {
          content_encoding: Some(BROTLI.into()),
          ..inscription("text/plain;charset=utf-8", "HELLOWORLD")
        },
        id: inscription_id(1),
        number: 1,
        satpoint: satpoint(1, 0),
        output: Some(TxOut {
          value: Amount::from_sat(1),
          script_pubkey,
        }),
        ..default()
      },
      "
        <h1>Inscription 1</h1>
        .*
        <dl>
          .*
          <dt>burn metadata</dt>
          <dd>
            <dl><dt>foo</dt><dd>bar</dd><dt>baz</dt><dd>1</dd></dl>
          </dd>
          .*
        </dl>
      "
      .unindent()
    );
  }

  #[test]
  fn opus_metadata_key_normalization_ordering_and_picture_filtering() {
    let metadata = opus_metadata::from_tags(vec![
      ("METADATA_BLOCK_PICTURE".into(), "base64-data".into()),
      ("ALBUM_ARTIST".into(), "Album Artist".into()),
      ("TRACKNUMBER".into(), "3".into()),
      ("Technology".into(), "DAW".into()),
      ("CustomTag".into(), "custom value".into()),
      ("TITLE".into(), "Song Title".into()),
    ])
    .unwrap();

    assert_eq!(
      metadata,
      Value::Map(vec![
        (
          Value::Text("title".into()),
          Value::Text("Song Title".into())
        ),
        (
          Value::Text("album artist".into()),
          Value::Text("Album Artist".into())
        ),
        (Value::Text("track number".into()), Value::Text("3".into())),
        (Value::Text("technology".into()), Value::Text("DAW".into())),
        (
          Value::Text("custom tag".into()),
          Value::Text("custom value".into())
        ),
      ])
    );
  }

  #[test]
  fn opus_metadata_extracts_tags_from_real_opus_file() {
    let metadata = InscriptionHtml {
      inscription: inscription(
        "audio/ogg;codecs=opus",
        include_bytes!("../../testdata/comingsoon.opus"),
      ),
      ..default()
    }
    .opus_metadata()
    .unwrap();

    let Value::Map(fields) = metadata else {
      panic!("expected opus metadata map");
    };

    assert_eq!(text_field(&fields, "title"), Some("comingsoon"));
    assert_eq!(text_field(&fields, "artist"), Some("Tatiana Moroz"));
    assert_eq!(text_field(&fields, "album artist"), Some("Tatiana Moroz"));
    assert_eq!(text_field(&fields, "track number"), Some("1"));
    assert_eq!(text_field(&fields, "technology"), Some("Michael Evans"));
    assert!(
      !fields
        .iter()
        .any(|(key, _)| matches!(key, Value::Text(key) if key.contains("picture")))
    );
  }

  fn text_field<'a>(fields: &'a [(Value, Value)], name: &str) -> Option<&'a str> {
    fields.iter().find_map(|(key, value)| {
      if matches!(key, Value::Text(key) if key == name) {
        if let Value::Text(value) = value {
          return Some(value.as_str());
        }
      }

      None
    })
  }

  #[test]
  fn metadata_sections_display_opus_metadata_before_metadata() {
    let sections = InscriptionHtml::metadata_sections_from_values(
      Some(Value::Map(vec![(
        Value::Text("title".into()),
        Value::Text("opus".into()),
      )])),
      Some(Value::Map(vec![(
        Value::Text("foo".into()),
        Value::Text("bar".into()),
      )])),
    );

    assert_eq!(
      sections,
      vec![
        (
          "opus metadata",
          Value::Map(vec![(
            Value::Text("title".into()),
            Value::Text("opus".into())
          )]),
        ),
        (
          "metadata",
          Value::Map(vec![(Value::Text("foo".into()), Value::Text("bar".into()))]),
        ),
      ]
    );
  }
}
