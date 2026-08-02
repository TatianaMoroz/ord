// Off-chain marketplace links for ordinals.gallery collections.
//
// This is intentionally NOT derived from on-chain data. Marketplaces use
// different slugs for the same collection (e.g. Bitcoin Rocks is "bitcoinrocks"
// on Satflow but "bitcoin-rocks" on Ordinals Wallet), so we track each
// marketplace's slug per gallery by hand here. This file is the entire data
// layer for the feature — index.js reads it to render the marketplace dropdown.
// Nothing here touches ord's Rust/server/index code, so it survives upstream
// ordinals.com updates.

// Marketplace URL prefixes. Only the trailing slug varies per collection.
const MARKETPLACES = {
  satflow:        { name: 'Satflow',         base: 'https://www.satflow.com/ordinals/' },
  ordnet:         { name: 'Ordnet',          base: 'https://ord.net/collection/' },
  ordinalswallet: { name: 'Ordinals Wallet', base: 'https://ordinalswallet.com/collection/' },
};

// Gallery (collection) inscription id -> { marketplaceKey: slug }.
// Omit a marketplace if the collection isn't listed there; the dropdown skips it.
const GALLERY_MARKETPLACES = {
  // Bitcoin Shrooms
  'e8397d69efd07b1e8ba995e5d138b66ca1d87910ee5f0001b6ab50693d8f5f0ei0': {
    satflow: 'bitcoinshrooms',
    ordnet: 'bitcoinshrooms',
    ordinalswallet: 'bitcoinshrooms',
  },
  // Bitcoin Rocks — slugs differ per marketplace; Ordnet slug unknown (omitted)
  'BITCOIN_ROCKS_INSCRIPTION_ID': {
    satflow: 'bitcoinrocks',
    ordinalswallet: 'bitcoin-rocks',
  },
};
