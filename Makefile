BUILD_DIR :=./out/Nightly
NIGHTLY_REPORT_DIR:=./skia_opt_research/out/$(shell date +'%Y-%m-%d_%H-%M-%S')
REPORT_GENERATOR=python3 ./skia_opt_research/gen_report.py
REPORT_TEMPLATE=./skia_opt_research/report_template.html
SKP_DIR=./skia_opt_research/skps
SKPS := $(shell ls -d ./skia_opt_research/skps/*.skp)
JSON := $(shell ls -d ./skia_opt_research/skps/*.json)
SKI_OPT_DIR=./skia_opt_research/SkiOpt
SKI_OPT_BIN=./skia_opt_research/SkiOpt/target/release/ski_opt


clean:
	$(RM) -r $(BUILD_DIR)
	$(RM) -r $(SKP_DIR)
	cargo clean --manifest-path=$(SKI_OPT_DIR)/Cargo.toml --release

gen-nightly:
	python3 ./tools/git-sync-deps
	./bin/gn gen $(BUILD_DIR) --args='is_official_build=false skia_enable_malloc_logging=true'

build-skiopt: 
	cargo build --manifest-path=$(SKI_OPT_DIR)/Cargo.toml --release

build-nightly: gen-nightly
	ninja -C $(BUILD_DIR) skia_opt_membench
	ninja -C $(BUILD_DIR) skia_opt_gen_skps
	ninja -C $(BUILD_DIR) skp_parser

gen-skps: build-nightly
	mkdir -p $(SKP_DIR)
	$(BUILD_DIR)/skia_opt_gen_skps

gen-skp-json: gen-skps
	for SKP in $(SKPS); do $(BUILD_DIR)/skp_parser $${SKP} > $${SKP}.json; done
	@@echo "Generated JSON representations of SKPs": $(JSON)

gen-skiopt-skps: gen-skp-json build-skiopt
	for SKP in $(SKPS); do $(SKI_OPT_BIN) $${SKP}.json >> $${SKP}.skilang.txt; done
	@@echo "Generated JSON representations of SKPs": $(JSON)

nightly-dry:
	@@echo mkdir -p $(NIGHTLY_REPORT_DIR)
	@@echo $(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_OUT_DIR)
	@@echo scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia

list-skps:
	@@echo "List of SKPS: " $(SKPS)

nightly: list-skps clean build-nightly gen-skiopt-skps
	mkdir -p $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skia_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_REPORT_DIR)
	$(REPORT_GENERATOR) -d $(NIGHTLY_REPORT_DIR) -t $(REPORT_TEMPLATE)
	scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia/
