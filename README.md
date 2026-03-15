# memocp

`memocp` is a blazing fast, stateful CLI written in Rust that decouples source and destination directories, by ensuring
a file is strictly copied once based on its cryptographic hash.

## Usage

```
Usage: memocp.exe [OPTIONS] <SOURCE_PATH> <DESTINATION_PATH>

Arguments:
  <SOURCE_PATH>       The source directory/file to copy from
  <DESTINATION_PATH>  The destination/file directory to copy to. If the directory does not exist, it will be created

Options:
      --glob <GLOB>
          The glob pattern to use for filtering files. Ignored if the source path is a file. Globs are matched case-insensitively
  -s, --state-file <STATE_FILE>
          The state file to use for memoization [default: ./memocp.db]
  -v, --verbose
          Enable verbose logging
      --concurrency <CONCURRENCY>
          The maximum number of threads to use for hashing and copying. An additional thread will always be used for scanning. Defaults to `8` or the number of CPU cores, whichever is smaller [default: 8]
      --exclusive-lock
          Take an exclusive lock on files during hashing. You likely only want to use this under Windows, where file locking is more reliable
      --hashing-read-chunk-size <HASHING_READ_CHUNK_SIZE>
          The number of bytes to read at a time when hashing files, per thread. Supports units like "KiB", "MiB", "GiB", etc [default: "4 MiB"]
      --ignore-hidden
          Ignore hidden files
      --override
          Override existing files at the destination
      --mode <COPY_MODE>
          The copy mode to use [default: reflink] [possible values: hard-link, reflink, copy]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Template Syntax

The destination path can contain the following variables:

| Variable        | Description                                              |
|-----------------|----------------------------------------------------------|
| `{year_utc}`    | Year the file was last modified, in UTC.                 |
| `{month_utc}`   | Month the file was last modified, in UTC.                |
| `{day_utc}`     | Day the file was last modified, in UTC.                  |
| `{year_local}`  | Year the file was last modified, in the local timezone.  |
| `{month_local}` | Month the file was last modified, in the local timezone. |
| `{day_local}`   | Day the file was last modified, in the local timezone.   |

## Considerations

- Directory symlinks are followed, file symlinks are not copied (the symlinked file is copied).
- Hidden files are considered by default.
- When copying is not atomic, `memocp` will write to a hidden temporary file and then rename the file to the final
  destination.
- When the copy mode is `reflink` and reflinking fails (e.g., files are on different filesystems, the filesystem does
  not support reflinking, etc.), `memocp` will fall back to copying the file.
