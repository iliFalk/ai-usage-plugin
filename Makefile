PREFIX ?= /usr/local

.PHONY: build install uninstall test plugin-test omarchy-visual-test changelog-check \
	clippy fmt clean

build:
	cargo build --release

install: build
	install -Dm755 -- target/release/ai-usagebar     "$(DESTDIR)$(PREFIX)/bin/ai-usagebar"
	install -Dm644 -- config.example.toml            "$(DESTDIR)$(PREFIX)/share/ai-usagebar/config.example.toml"
	install -Dm644 -- README.md                      "$(DESTDIR)$(PREFIX)/share/doc/ai-usagebar/README.md"
	install -Dm644 -- LICENSE                        "$(DESTDIR)$(PREFIX)/share/licenses/ai-usagebar/LICENSE"

uninstall:
	rm -f -- "$(DESTDIR)$(PREFIX)/bin/ai-usagebar"
	rm -rf -- "$(DESTDIR)$(PREFIX)/share/ai-usagebar"
	rm -rf -- "$(DESTDIR)$(PREFIX)/share/doc/ai-usagebar"
	rm -rf -- "$(DESTDIR)$(PREFIX)/share/licenses/ai-usagebar"

test:
	cargo test
	$(MAKE) plugin-test

changelog-check:
	./scripts/check-changelog-immutable.sh

# The panel's whole frontend contract: the report model, the settings patch and
# the QML source contracts. Bare node, so it needs nothing but a Node runtime.
plugin-test:
	node omarchy/model.test.mjs

# Renders the real SettingsView through Quickshell's offscreen backend. Opt-in:
# non-Omarchy machines have neither Quickshell nor the shell's qs.Commons/qs.Ui
# modules installed.
omarchy-visual-test:
	./scripts/omarchy-visual-test.sh

clippy:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
