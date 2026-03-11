# memocp

`memocp` is a blazing fast, stateful CLI written in Rust that decouples source and destination directories, by ensuring
a file is strictly copied once based on its cryptographic hash.

## Considerations

- Directory symlinks are followed, file symlinks are not copied.
- Hidden files are considered by default.
- 