# Copr's "make_srpm" build method runs `make -f .copr/Makefile srpm outdir=...`.
# It packs this checkout into a tarball, puts the Cargo.toml version into the
# spec, and builds the source rpm. The binary rpm is then built from it
# (packaging/linux/dotnote.spec).

top := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
outdir ?= $(top)/_srpm
version := $(shell sed -n 's/^version = "\(.*\)"/\1/p' $(top)/Cargo.toml | head -1)
work := $(top)/_copr

srpm:
	dnf -y install git rpm-build
	git config --global --add safe.directory '*'
	rm -rf $(work) && mkdir -p $(work) $(outdir)
	git -C $(top) archive --format=tar.gz --prefix=dotnote-$(version)/ \
		-o $(work)/dotnote-$(version).tar.gz HEAD
	sed 's/@VERSION@/$(version)/g' $(top)/packaging/linux/dotnote.spec > $(work)/dotnote.spec
	rpmbuild -bs $(work)/dotnote.spec \
		--define "_sourcedir $(work)" --define "_srcrpmdir $(outdir)"
