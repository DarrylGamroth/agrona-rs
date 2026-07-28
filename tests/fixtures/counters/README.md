# Agrona Java counter fixture

`metadata.bin` and `values.bin` are produced by
`tests/interop/java/CountersReaderFixtureGenerator.java` using Agrona Java
commit `d4a47c67258f85b39910c4999da346ead655b736`.

Regenerate from a checkout at that exact revision:

```text
python3 scripts/generate_counters_reader_java_fixture.py \
  --agrona-root /path/to/agrona \
  --output tests/fixtures/counters \
  --bidirectional
```

CI checks out the pinned revision, regenerates both files with Java 17,
compares them byte for byte with the committed fixtures, reads them with
Rust, generates a second set with the Rust manager, and validates that set
with the actual pinned Agrona Java `CountersReader`.
