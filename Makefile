.PHONY: run
run: test
	cargo run

.PHONY: test
test: lint


.PHONY: lint
lint:
	cargo check

.PHONY: build
build:
	cargo build --release
