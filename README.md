# small-axe - [WIP] \*nix tree command in Rust

“If you are the big tree, we are the small axe.”


```bash
tree [-adfghilnoprstuCDFN] [-L level] [-P pattern] [-I pattern] [--inodes] [--device] [--noreport] [--dirsfirst] [--help] [directory ...]
```

- Additional flags are under active development

<img alt="Tree output in terminal image" src="./static/tree.webp" width="200" />

Borrowing iterator logic was heavily inspired by [Walkdir](https://docs.rs/walkdir/latest/walkdir/).
