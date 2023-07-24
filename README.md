Colight
=======

![screenshot](docs/screenshot.webp)

Command line utility that colors text based on how compressible it is.
Compressibility is determined using a lz77-like algorithm.
The idea is that compressibility is a good indicator of whether a piece of text is interesting or not.

It only works in some terminals.

## Usage

```bash
$ cat file | colight
```

