.PHONY: build install clean

build:
	cargo build --release
	upx --best --lzma target/release/nyalog

install: build
	cp target/release/nyalog ~/.cargo/bin/

clean:
	cargo clean
