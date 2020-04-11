SHELL := /bin/sh

include ../../config.mk

rust-examples:
	cargo build $(BUILD_MODE) --target-dir=$(OUT_DIR)
	ln -sf $(OUT_DIR)/$(TARGET_NAME)/rust-example $(OUT_DIR)/
	

clean:
	cargo clean