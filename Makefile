# Disable all the default make stuff
MAKEFLAGS += --no-builtin-rules
.SUFFIXES:

IMAGE_FILES = $(shell find $(PWD)/resources -type f -name *.jpeg) $(shell find $(PWD)/resources  -type f -name *.png) $(shell find $(PWD)/resources  -type f -name *.jpg)
WEBP_FILES = $(addsuffix .webp,$(IMAGE_FILES))

## Display help menu
.PHONY: help
help:
	@echo Documented Make targets:
	@perl -e 'undef $$/; while (<>) { while ($$_ =~ /## (.*?)(?:\n# .*)*\n.PHONY:\s+(\S+).*/mg) { printf "\033[36m%-30s\033[0m %s\n", $$2, $$1 } }' $(MAKEFILE_LIST) | sort

# ------------------------------------------------------------------------------
# NON-PHONY TARGETS
# ------------------------------------------------------------------------------

$(WEBP_FILES): $(basename $@)
	cwebp $(basename $@) -o $@ -q 80

.score-compose/:
	score-compose init

# ------------------------------------------------------------------------------
# PHONY TARGETS
# ------------------------------------------------------------------------------

.PHONY: .ALWAYS
.ALWAYS:

## Generate webp variants of all images
.PHONY: generate-webp
generate-webp: $(WEBP_FILES)

## Check and test
.PHONY: test
test:
	cargo check && cargo fmt && cargo test

## Build development binaries
.PHONY: build
build:
	cargo build

## Setup and launch locally using score-compose
.PHONY: launch
launch: .score-compose/
	# TODO: get rid of this override prop here
	score-compose generate score.yaml --build web=. --override-property=resources.route.params.host=localhost
	EXTERNAL_URL_SCHEME=http:// EXTERNAL_URL_PORT=:8080 docker compose up --build
