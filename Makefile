# LP-0005 attestation gate — build / IDL / deploy workflow.
# Mirrors the convention used by lez-multisig and whisper-wall.
#
# Usage:
#   make setup            # bring up the local LEZ sequencer + wallet
#   make build            # compile the guest binary
#   make idl              # generate attestation-idl.json from the macros
#   make deploy           # publish the program; writes the id to state
#   make cli ARGS="..."   # invoke the IDL-driven CLI
#   make demo             # end-to-end: build, idl, deploy, demo flow

SHELL := /bin/bash
STATE_FILE := .attestation-state
IDL_FILE := attestation-idl.json
PROGRAMS_DIR := methods/guest/target/riscv32im-risc0-zkvm-elf/docker
PROGRAM_BIN := $(PROGRAMS_DIR)/attestation.bin

-include $(STATE_FILE)

define save_var
	@grep -v '^$(1)=' $(STATE_FILE) 2>/dev/null > $(STATE_FILE).tmp || true
	@echo '$(1)=$(2)' >> $(STATE_FILE).tmp
	@mv $(STATE_FILE).tmp $(STATE_FILE)
endef

.PHONY: help setup build idl cli deploy inspect demo status clean test precheck bundle

help: ## Show this help
	@echo "LP-0005 attestation gate"
	@echo ""
	@echo "  make setup      Bring up local sequencer + wallet (logos-scaffold)"
	@echo "  make build      Compile guest (cargo + risc0)"
	@echo "  make idl        Generate IDL from #[lez_program] macros"
	@echo "  make deploy     Deploy program; saves program id to $(STATE_FILE)"
	@echo "  make cli ARGS=  Run the IDL-driven CLI"
	@echo "  make inspect    Print the program id for the built binary"
	@echo "  make demo       Full end-to-end demo (RISC0_DEV_MODE=0 required)"
	@echo "  make status     Show saved state + binary info"
	@echo "  make test       Run host-side unit tests"
	@echo "  make clean      Remove saved state"

setup:
	logos-scaffold setup
	logos-scaffold localnet start
	logos-scaffold doctor

build:
	cargo build -p attestation_methods --release

idl:
	cargo run -p attestation-idl-gen > $(IDL_FILE)
	@echo "IDL written to $(IDL_FILE)"

cli:
	logos-scaffold spel -- $(ARGS)

deploy: build idl
	@PID=$$(logos-scaffold deploy --program-path $(PROGRAM_BIN) --json | jq -r '.program_id'); \
	echo "deployed: $$PID"; \
	$(call save_var,PROGRAM_ID,$$PID)

inspect:
	logos-scaffold spel -- inspect $(PROGRAM_BIN)

demo: build idl
	@[ "$(RISC0_DEV_MODE)" = "0" ] || (echo "abort: RISC0_DEV_MODE must be 0 for the demo" && exit 1)
	bash scripts/demo.sh

status:
	@echo "STATE_FILE: $(STATE_FILE)"
	@cat $(STATE_FILE) 2>/dev/null || echo "(no saved state)"
	@echo "PROGRAM_BIN: $(PROGRAM_BIN)"
	@[ -f $(PROGRAM_BIN) ] && ls -la $(PROGRAM_BIN) || echo "(not built)"

test:
	cargo test -p attestation_core -p attestation_verifier

# Same checks CI runs, so a green precheck means a green CI push.
precheck:
	cargo fmt --all -- --check
	cargo clippy -p attestation_core -p attestation_verifier -p attestation_program --all-targets -- -D warnings
	cargo test -p attestation_core -p attestation_verifier -p attestation_program
	cd ts && npx tsc --noEmit && cd ..
	cd examples/chat-gate && npx tsc --noEmit && cd ../..
	cd examples/fee-tier-gate && npx tsc --noEmit && cd ../..
	@echo "✓ precheck clean — safe to push"

bundle:
	@rm -f basecamp-app.zip
	cd basecamp-app && zip -qr ../basecamp-app.zip . -x '*.DS_Store'
	@echo "✓ basecamp-app.zip — sideloadable in Basecamp"

clean:
	rm -f $(STATE_FILE) $(IDL_FILE) basecamp-app.zip
