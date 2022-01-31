# Changelog

## 0.3.0

### Features

- Tile decoding is now implemented.

### Fixes

- Some of the DMRE sizes did not encode (panic) due to a bug in the
  `placement` module.

### Breaking changes

- Rust 2021 is now used.
- `DataMatrix::codewords` now also returns the error correction codewords.
  Use `DataMatrix::data_codewords` instead.
- Any type implementing the `Bit` trait now needs to be `Copy`.
- `traverse` is now called `traverse_mut` and no longer writes the padding pattern.
  Use `write_padding` to do that.