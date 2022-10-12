BUILD_DIR :=./out/Nightly
NIGHTLY_OUT_DIR:=./skia_opt_research/out/$(shell date +'%Y-%m-%d_%H-%M-%S')
SKPS := $(shell ls -d $(PWD)/skia_opt_research/skps/*)

clean:
	$(RM) -r $(BUILD_DIR)

gen-nightly:
	python3 ./tools/git-sync-deps
	./bin/gn gen $(BUILD_DIR) --args='is_official_build=false skia_enable_malloc_logging=true'

build-nightly: gen-nightly
	ninja -C $(BUILD_DIR) skp_opt_membench

nightly-dry:
	@@echo mkdir -p $(NIGHTLY_OUT_DIR)
	@@echo $(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_OUT_DIR)
	@echo scp -r -C $(NIGHTLY_OUT_DIR) uwplse.org:/var/www/skia

nightly: clean build-nightly
	mkdir -p $(NIGHTLY_OUT_DIR)
	$(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_OUT_DIR)
	scp -r -C $(NIGHTLY_OUT_DIR) uwplse.org:/var/www/skia/
