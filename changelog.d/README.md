# Changelog fragments

A change people will notice adds its entry to the changelog here, as a
file, rather than to [CHANGELOG.md](../CHANGELOG.md). Pull requests open at
the same time then never change the same lines, and never conflict over
the changelog.

Put the file in the folder of its section, as
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) names them:
`added`, `changed`, `deprecated`, `removed`, `fixed` or `security`. Name it
after the issue it closes, such as `fixed/101-audio-stutter.md`. The issue's
number orders the entries. The file holds the entry as CHANGELOG.md has
them: a Markdown list item, or a few, wrapped near 74 columns.

```md
- A game that stops sending audio partway through a track no longer
  freezes its window (#101).
```

Write for the person who plays: say what they will see change, not which
module changed.

Three commands work on the fragments:

```sh
cargo xtask changelog --check            # CI runs this: are the fragments valid?
cargo xtask changelog --print            # show ## [Unreleased] as the next release will have it
cargo xtask changelog --release 0.2.0    # fold them into ## [0.2.0] - <today> and delete them
```

`--release` takes an optional `--date YYYY-MM-DD`. It adds the fragments
after the entries already under `## [Unreleased]`, renames that section to
the version, and puts a new empty `## [Unreleased]` above it. The release
commit is made from its result.
