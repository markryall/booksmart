# booksmart

A terminal bookmark launcher. Type to filter your bookmarks like fzf, press
Enter, and the bookmark opens in the right browser space or profile.

Bookmarks live in a TOML file you can edit by hand, and booksmart picks up
changes to it as they happen.

```toml
[mail-work]
space = "Work"
url = "https://mail.example.com"

[docs]
space = "Home"
url = ["https://docs.example.com", "https://wiki.example.com"]   # open together
tags = ["dev"]

[order]
space = "Work"
url = "https://shop.example.com/orders/{1}"                       # asks for {1}

[search]
space = "Home"
url = "https://www.google.com/search?q={*}"                       # {*} takes the rest
```

## Install

```sh
brew install markryall/tap/booksmart
```

or, with a Rust toolchain:

```sh
cargo install booksmart
```

## Keys

| key | does |
|---|---|
| type | filter the list (fuzzy) |
| ↑ ↓ | select |
| Enter | open the bookmark, asking first for any `{1}` `{2}`.. `{*}` values |
| Ctrl-N | new bookmark, picking its space from the browser's |
| Ctrl-E | edit the selected bookmark |
| Ctrl-U | clear a field |
| Esc | clear the filter, or quit |

New and edited bookmarks are written back without disturbing anything else
in the file, comments included.

## Config

`~/.config/booksmart/config.toml`, or wherever `$BOOKSMART_CONFIG` points:

```toml
bookmarks = "~/.config/booksmart/bookmarks.toml"   # the default

[browser]
type = "crest"
```

Browsers:

- **crest**: [Crest](https://github.com/pauljoda/Crest) spaces, through the
  MCP server bundled with Crest. Allow the spaces you want in Crest's
  Settings › Agents.
- **command**: any command, run once per url with `{space}` and `{url}`
  filled in. `spaces` lists what the new-bookmark form offers. Without a
  `[browser]` section, booksmart uses `command = ["open", "{url}"]`, which
  opens urls in your default browser and ignores spaces.

  ```toml
  [browser]
  type = "command"
  command = ["my-opener", "--profile", "{space}", "{url}"]
  spaces = ["Work", "Home"]
  ```

## Building

```sh
mise install
mise run test
mise run release   # target/release/booksmart
```

Work is tracked as [cardthing](https://github.com/markryall/cardthing) cards in `.cards/`.
