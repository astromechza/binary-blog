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

# ------------------------------------------------------------------------------
# PHONY TARGETS
# ------------------------------------------------------------------------------

.PHONY: .ALWAYS
.ALWAYS:

## Generate webp variants of all images
.PHONY: generate-webp
generate-webp: $(WEBP_FILES)
