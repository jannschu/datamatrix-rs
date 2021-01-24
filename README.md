# datamatrix-rs

Data Matrix (ECC 200) decoding and encoding library with an optimizing encoder.

This library is still work in progress but a complex, important part is already
done: a new optimzing encoder for the data part. This optimizer goes beyond what
is specified in ISO/IEC 16022:2006. It uses an idea similar to the A\*
algorithm. The full encoding process is linear in the input size and therefore
beats the algorithm from the standard also on this front (this is probably not
noticable in most cases, though).  The new optimizer is unique about this
implementation, something similar could not be found in any open source
implementation.

It is also a completely fresh implementation overall, with all its pros and cons.

See the list of related projects below for credits.

## Status

- [x] Encodation modes ASCII, Base256, C40, Text, X12, EDIFACT implemented.
- [x] Optimizer for switching between encodation modes to find a minimal
      encodation size.
- [x] Data part decoding.
- [x] Fuzz decoding and encoding (*never complete, but no issues after 48h*)
- [x] Check the open bug reports in other implementations.
- [x] Reed Solomon error code creation.
- [ ] Reed Solomon correction.
- [ ] Tile (aka. module) placement encoding.
- [ ] Tile (aka. module) placement decoding.

After the above steps the library will be useable for full Data Matrix
encoding and decoding (from an end user perspective). No rendering or virtual
detection will be there yet, but at least rendering is straightforward.

- [ ] Helpers for rendering
- [ ] Visual detection in images

Things in consideration for after that:

- Refine API for better symbol size control ("at least 14x14" for example)
- ECI support (UTF-8 for example)
- "Structured Append"
- "Reader Programming"
- FCN1 and GS1

## Related projects

The following projects were invaluable for learning from their implementation
and stealing some of their test cases and bug reports.

- [zxing](https://github.com/zxing/zxing) is a Google library to encode
  and decode multiple 1D and 2D codes including Data Matrix. The core part
  is written in Java.
- [barcode4j](http://barcode4j.sourceforge.net/) is a predecessor of zxing,
  the Data Matrix code was forked into zxing and improved.
- [libdmtx](https://github.com/dmtx/libdmtx) is the most promiment open source
  C library for encoding and decoding Data Matrix. It has a limited but
  very useable optimizer for the encoding.
- [zxing-cpp](https://github.com/nu-book/zxing-cpp) is a C++ port of zxing, it
  also contains some improvements.