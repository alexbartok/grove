PREFIX ?= /usr/local
BINARY = grove

.PHONY: build install uninstall clean

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
