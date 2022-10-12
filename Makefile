BUILD_DIR :=./out/Nightly
NIGHTLY_REPORT_DIR:=./skia_opt_research/out/$(shell date +'%Y-%m-%d_%H-%M-%S')
REPORT_GENERATOR=python3 ./skia_opt_research/gen_report.py
REPORT_TEMPLATE=./skia_opt_research/report_template.html
SKPS := $(shell ls -d ./skia_opt_research/skps/*)

clean:
	$(RM) -r $(BUILD_DIR)

gen-nightly:
	python3 ./tools/git-sync-deps
	./bin/gn gen $(BUILD_DIR) --args='is_official_build=false skia_enable_malloc_logging=true'

build-nightly: gen-nightly
	ninja -C $(BUILD_DIR) skp_opt_membench

nightly-dry:
	@@echo mkdir -p $(NIGHTLY_REPORT_DIR)
	@@echo $(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_OUT_DIR)
	@@echo scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia

list-skps:
	@@echo "List of SKPS: " $(SKPS)

nightly: list-skps clean build-nightly
	mkdir -p $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_REPORT_DIR)
	$(REPORT_GENERATOR) -d $(NIGHTLY_REPORT_DIR) -t $(REPORT_TEMPLATE)
	scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia/
