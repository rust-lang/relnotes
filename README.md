# relnotes: Generate release notes for "The Rust Programming Language"
This utility pulls all pull requests made against `rust-lang/rust` and
`rust-lang/cargo` within the latest release cycle and prints out a markdown
document containing all the pull requests, categorised into their respective
sections where possible, and prints the document to `stdout`.


## Usage
`version_number` is the version number of the rust release. e.g. `1.28.0`
```
cargo run --release <version_number> > release.md
```
