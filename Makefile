PROTOC=/usr/bin/protoc

BUILD_DIR :=./out/Nightly
NIGHTLY_REPORT_DIR:=./skia_opt_research/out/$(shell date +'%Y-%m-%d_%H-%M-%S')
REPORT_GENERATOR=python3 ./skia_opt_research/gen_report.py
REPORT_TEMPLATE=./skia_opt_research/report_template.html
SKP_DIR=./skia_opt_research/skps
SKPS = $(wildcard ./skia_opt_research/skps/*.skp)

SKI_PASS_DIR=./skia_opt_research/SkiPass
SKI_PASS_BIN=./skia_opt_research/SkiPass/target/release/ski_pass
PROTO_SRC_DIR=./skia_opt_research/protos
PROTO_CPP_GEN_DIR=./skia_opt_research/
PROTO_PY_GEN_DIR=./skia_opt_research/
PROTOS = $(wildcard ./skia_opt_research/protos/*.proto)

SKP_RENDERS = $(NIGHTLY_REPORT_DIR)/renders
SKI_PASS_SKP_RENDERS = $(NIGHTLY_REPORT_DIR)/skipass_renders
DIFF_REPORT_DIR = $(NIGHTLY_REPORT_DIR)/diff

export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION = python


gen-proto:
	${PROTOC} -I=${PROTO_SRC_DIR} --cpp_out=${PROTO_CPP_GEN_DIR} --python_out=${PROTO_PY_GEN_DIR} ${PROTOS}

clean:
	$(RM) -r $(BUILD_DIR)
	$(RM) -r $(SKP_DIR)
	cargo clean --manifest-path=$(SKI_PASS_DIR)/Cargo.toml --release

gen-nightly:
	python3 ./tools/git-sync-deps
	./bin/gn gen $(BUILD_DIR) --args='is_official_build=false skia_enable_malloc_logging=true'

build-skipass: 
	cargo build --manifest-path=$(SKI_PASS_DIR)/Cargo.toml --release

build-nightly: gen-nightly gen-proto
	ninja -C $(BUILD_DIR) skia_opt_membench
	ninja -C $(BUILD_DIR) skia_opt_gen_skps
	ninja -C $(BUILD_DIR) skp_parser
	ninja -C $(BUILD_DIR) skdiff

gen-skps: build-nightly
	mkdir -p $(SKP_DIR)
	$(BUILD_DIR)/skia_opt_gen_skps
	cp ${SKP_DIR}/webpages/* ${SKP_DIR}/

gen-skp-json: gen-skps
	for SKP in $(SKPS); do $(BUILD_DIR)/skp_parser $${SKP} > $${SKP}.json; done

gen-skiopt-skps: gen-skp-json build-skipass
	# For each SKP, using it's JSON, generate a optimized SKP using SkiOpt
	# The optimized SKPs are stored with a file extension of .skp_opt - these are just normal skps generated by our optimizer.
	for SKP in $(SKPS); do $(SKI_PASS_BIN) $${SKP}.json $${SKP}_opt > $${SKP}_skiopt_debug_log.txt || continue; done

nightly-dry:
	@@echo mkdir -p $(NIGHTLY_REPORT_DIR)
	@@echo $(BUILD_DIR)/skp_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_OUT_DIR)
	@@echo scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia


local-nightly: build-nightly gen-skiopt-skps
	mkdir -p $(SKP_RENDERS)
	mkdir -p $(SKI_PASS_SKP_RENDERS)
	mkdir -p $(DIFF_REPORT_DIR)
	mkdir -p $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skia_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skdiff $(SKP_RENDERS) $(SKI_PASS_SKP_RENDERS) $(DIFF_REPORT_DIR)
	$(REPORT_GENERATOR) -d $(NIGHTLY_REPORT_DIR) -t $(REPORT_TEMPLATE)
	cp $(SKP_DIR)/* $(NIGHTLY_REPORT_DIR)

nightly: clean local-nightly
	scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia/
