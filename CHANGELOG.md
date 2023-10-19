# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.2.1] - 2023-10-19
- Fixed Tree Borrows and Stacked Borrows violations when running `cargo miri` (#6, #7)

## [0.2.0] - 2022-10-27
- Fixed double-drop when a Drop handler of an element panics during `clear`
- Added an `Extend` implementation for `NtBoxingListHead`
- Added a `FromIterator` implementation for `NtBoxingSingleListHead`
- Added documentation which structures are only available behind the `alloc` feature (#1)
- Fixed undefined behavior caused by incorrect usage of `MaybeUninit` (#2, #3)
- Changed all type casts to convert between pointers directly without going via `usize` (#4)
- Changed `NtListElement` to an `unsafe trait` to prevent possibly undefined behavior from safe code (#5)
- Changed all code examples in the documentation to be buildable as doc-tests
- Fixed warnings emitted by clippy of Rust 1.64.0

## [0.1.0] - 2022-07-28
- Initial release
