# Introduction

`subparse` is a Rust library that lets use load, change and store subtitle files in various formats. Formatting and other data will be preserved.

You can find an examples how to use this library under `examples/`.

Currently supported are:

-   SubStationAlpha `.ssa`/`.ass`
-   MicroDVD `.sub`
-   SubRip `.srt`
-   VobSub `.idx` and `.sub`

[Documentation](https://docs.rs/subparse)

[Crates.io](https://crates.io/crates/subparse)

## How to use the library
Add this to your `Cargo.toml`:

```toml
[dependencies]
subparse = "~0.1.0"
```
