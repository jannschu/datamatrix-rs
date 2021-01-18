# datamatrix-rs

Data Matrix (ECC 200) decoding and encoding library with an optimizing encoder.

This library is still work in progress but a complex, important part is already done:
an optimzing encoder for the data part. This optimizer goes beyond what is specified in
ISO/IEC 16022:2006. It uses an idea similar to the A\* algorithm. This is unique
about this implementation, something similar could not be found in any open source
implementation. It is also a completely fresh take overall -- with all its pros and cons.

See the list of related projects below for credits.

## Status

- [x] Encodation modes ASCII, Base256, C40, Text, X12, EDIFACT implemented.
- [x] Optimizer for switching between encodation modes to find a minimal
      encodation size.
- [ ] Fuzz decoding and encoding (**in progress**)
- [ ] Check open bug reports in other implementations
- [ ] Reed Solomon code creation
- [ ] Reed Solomon error correction
- [ ] Tile placement encoding
- [ ] Tile placement decoding

After the above steps the library is firstly useable for full Data Matrix
encoding and decoding (from an end user perspective). No rendering or virtual
detection will be there yet, but at least rendering is  straightforward.

- [ ] Visual detection in images
- [ ] Helpers for rendering

## Related projects

The following projects were invaluable for learning from their implementation
and stealing some of their test cases and bug reports.

- [zxing](https://github.com/zxing/zxing) is a Google library to encode
  and decode multiple 1D and 2D codes including Data Matrix. The core part
  is written in Java.
- [barcode4j](http://barcode4j.sourceforge.net/) is the predecessor of zxing,
  the Data Matrix code was forked into zxing and improved.
- [libdmtx](https://github.com/dmtx/libdmtx) is the most promiment open source
  C library for encoding and decoding Data Matrix but it can not switch between
  encodations (ASCII, C40, ...) and only sticks to one for the full encodation.
- [zxing-cpp](https://github.com/nu-book/zxing-cpp) is a C++ port of zxing, it
  also contains some improvements.