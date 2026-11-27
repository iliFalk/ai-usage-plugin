<!--
Thanks for the contribution. The checklist below is what review would
otherwise ask for — see CONTRIBUTING.md for the reasoning behind each.
-->

## What this changes



## Screenshots

<!-- For a visual change (bar chip, popup, settings page): before and after,
for every locale and theme it touches. Delete this section otherwise. -->

## Checklist

- [ ] `make test`, `cargo clippy --all-targets -- -D warnings` and
      `cargo fmt --all -- --check` pass
- [ ] `CHANGELOG.md` entry under `## [Unreleased]` in the right category
      (not inside an already-released section)
- [ ] A test that fails on `main` and passes here, for a bug fix
- [ ] Docs updated where they mention what changed (`README.md`,
      `config.example.toml`, `docs/`)
- [ ] `make plugin-test` passes when `omarchy/` or `manifest.json` changed
- [ ] No helper left behind without callers

## Testing

<!-- What you ran. `make omarchy-visual-test` needs Quickshell; say so if the
visual check could not run. -->
