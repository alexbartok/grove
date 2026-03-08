PREFIX ?= /usr/local
BINARY = grove

.PHONY: build install uninstall clean

build:
	cargo build --release

install: build
	install -d $(PREFIX)/bin
	install -m 755 target/release/$(BINARY) $(PREFIX)/bin/$(BINARY)

uninstall:
	rm -f $(PREFIX)/bin/$(BINARY)

clean:
	cargo clean
