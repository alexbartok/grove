PREFIX ?= /usr/local
BINARY = grove

.PHONY: build install uninstall clean release

build:
	cargo build --release

install: build
	@if [ ! -w "$(PREFIX)/bin" ] && [ "$$(id -u)" -ne 0 ]; then \
		echo "Error: $(PREFIX)/bin is not writable. Either:"; \
		echo "  sudo make install"; \
		echo "  make install PREFIX=~/.local"; \
		exit 1; \
	fi
	install -d $(PREFIX)/bin
	install -m 755 target/release/$(BINARY) $(PREFIX)/bin/$(BINARY)

uninstall:
	rm -f $(PREFIX)/bin/$(BINARY)

clean:
	cargo clean

release:
	@VERSION=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	TAG="v$$VERSION"; \
	if git rev-parse "$$TAG" >/dev/null 2>&1; then \
		echo "Error: tag $$TAG already exists. Bump version in Cargo.toml first."; \
		exit 1; \
	fi; \
	echo "Releasing $$TAG"; \
	git tag "$$TAG" && \
	git push && \
	git push origin "$$TAG" && \
	echo "Done. GitHub Actions will build and publish the release."
