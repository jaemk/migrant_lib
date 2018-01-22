
# Contributing

Thanks for contributing!


## Getting Started

- [Install rust](https://www.rust-lang.org/en-US/install.html)
- Install database dependencies (linux): `apt install sqlite3 libsqlite3-dev postgresql libpq-dev`
- `cargo build`


## Making Changes

- Please be mindful of the feature gates used to implement functionality with and without database connection crates.
- After making changes, be sure to run the tests!
- This crate makes use of [`cargo-readme`](https://github.com/livioribeiro/cargo-readme) (`cargo install cargo-readme`)
  to generate the `README.md` from the crate level documentation in `src/lib.rs`.
  This means `README.md` should never be modified by hand.
  Changes should be made to the crate documentation in `src/lib.rs` and the `readme.sh` script run.


## Testing

The `test.sh` script exists to handle setup and tear-down of testing databases before running library tests,
as well as ensuring the tests are run with and without the various feature flags.
Note, some commands in this script will likely ask for your password when setting up managed databases, e.g. `postgres`.


## Submitting Changes

Pull Requests should be made against master.
Travis CI will run the test suite on all PRs.
Remember to update the changelog!

