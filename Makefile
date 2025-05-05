TARGET := x86_64-unknown-linux-gnu

.PHONY: run
run: test
	cargo run

.PHONY: test
test: lint


.PHONY: lint
lint:
	cargo check

.PHONY: clean
clean:
	rm -rf target/$(TARGET)

.PHONY: build
build:
	cross build --target $(TARGET) --release

.PHONY: upload_to_server
upload_to_server:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)
	@test -n "$(REMOTE_SERVER_PATH)" || (echo "Error: env REMOTE_SERVER_PATH is not set"; exit 1)
	scp target/$(TARGET)/release/friends-random-bot-rust $(REMOTE_SERVER_HOST):$(REMOTE_SERVER_PATH)

.PHONY: build_and_upload_to_server
build_and_upload_to_server: build upload_to_server
