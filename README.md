# relnotes: Generate release notes for "The Rust Programming Language"
This utility pulls all pull requests made against `rust-lang/rust` and
`rust-lang/cargo` within the latest release cycle and prints out a markdown
document containing all the pull requests, categorised into their respective
sections where possible, and prints the document to `stdout`.

## Requirements
`relnotes` uses the GitHub API to generate the release notes, as such you need
a valid GitHub API key. `relnotes` will look for `GITHUB_TOKEN` in the
environment and use that key when sending requests.

**small warning:** `relnotes` makes a lot of requests as GitHub only allows you to
look at 100 PRs in a single page. It is not recommended to call `relnotes`
multiple times as you can hit the GitHub's rate limit quite easily. Please refer
to [GitHub's Rate Limit documentation](https://developer.github.com/v4/guides/resource-limitations/#rate-limit) for more information.

## Usage
`version_number` is the version number of the rust release. e.g. `1.28.0`
```
cargo run --release <version_number> > release.md
```
