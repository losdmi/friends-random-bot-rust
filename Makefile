PROJECT := friends-random-bot-rust
BUILD_TARGET := x86_64-unknown-linux-gnu

.PHONY: run
run: test
	$(MAKE) run_no_lint

.PHONY: run_no_lint
run_no_lint:
	cargo run

.PHONY: test
test: lint
	cargo test

.PHONY: lint
lint:
	cargo clippy

.PHONY: clean
clean:
	rm -rf target/$(BUILD_TARGET)

.PHONY: build
build:
	cross build --target $(BUILD_TARGET) --release

.PHONY: deploy_config
deploy_config:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)
	@test -n "$(REMOTE_SERVER_PATH)" || (echo "Error: env REMOTE_SERVER_PATH is not set"; exit 1)

	$(MAKE) service_stop; true
	scp config.prod.json $(REMOTE_SERVER_HOST):$(REMOTE_SERVER_PATH)/config.json

.PHONY: deploy_to_server
deploy_to_server:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)
	@test -n "$(REMOTE_SERVER_PATH)" || (echo "Error: env REMOTE_SERVER_PATH is not set"; exit 1)

	$(MAKE) service_stop; true
	scp systemd.service $(REMOTE_SERVER_HOST):/etc/systemd/system/$(PROJECT).service
	ssh $(REMOTE_SERVER_HOST) "systemctl daemon-reload"
	scp target/$(BUILD_TARGET)/release/$(PROJECT) $(REMOTE_SERVER_HOST):$(REMOTE_SERVER_PATH)
	ssh $(REMOTE_SERVER_HOST) "systemctl enable $(PROJECT).service"
	$(MAKE) service_start

.PHONY: build_and_deploy_to_server
build_and_deploy_to_server: build deploy_to_server

.PHONY: service_stop
service_stop:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)

	ssh $(REMOTE_SERVER_HOST) "systemctl stop $(PROJECT).service"

.PHONY: service_start
service_start:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)

	ssh $(REMOTE_SERVER_HOST) "systemctl start $(PROJECT).service"

.PHONY: service_status
service_status:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)

	ssh $(REMOTE_SERVER_HOST) "systemctl status $(PROJECT).service"

.PHONY: service_watch
service_logs:
	@test -n "$(REMOTE_SERVER_HOST)" || (echo "Error: env REMOTE_SERVER_HOST is not set"; exit 1)

	ssh $(REMOTE_SERVER_HOST) "journalctl -u $(PROJECT) -f"

