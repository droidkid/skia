PROTOC=/usr/bin/protoc

BUILD_DIR :=./out/Nightly
NIGHTLY_REPORT_DIR:=./skia_opt_research/out/$(shell date +'%Y-%m-%d_%H-%M-%S')
REPORT_GENERATOR=python3 ./skia_opt_research/gen_report.py
REPORT_TEMPLATE=./skia_opt_research/report_template.html

# All skps will generated or copied over to ${SKPS}
WEBPAGE_SKPS_DIR = ./skia_opt_research/webpage_skps
SKPS = $(wildcard ./skia_opt_research/skps/*.skp)
SKP_DIR=./skia_opt_research/skps

SKI_PASS_DIR=./skia_opt_research/SkiPass
SKI_PASS_BUILD_DIR=./skia_opt_research/SkiPass/target/release
SKI_PASS_HEADER=$(realpath ./skia_opt_research/)/SkiPass.h

PROTO_SRC_DIR=./skia_opt_research/protos
PROTO_CPP_GEN_DIR=./skia_opt_research/
PROTO_PY_GEN_DIR=./skia_opt_research/
PROTOS = $(wildcard ./skia_opt_research/protos/*.proto)

SKP_RENDERS = $(NIGHTLY_REPORT_DIR)/renders
SKI_PASS_SKP_RENDERS = $(NIGHTLY_REPORT_DIR)/skipass_renders
DIFF_REPORT_DIR = $(NIGHTLY_REPORT_DIR)/diff

export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION = python
export SKI_PASS_LIB_DIR = $(realpath ${SKI_PASS_BUILD_DIR})

gen-proto:
	${PROTOC} -I=${PROTO_SRC_DIR} --cpp_out=${PROTO_CPP_GEN_DIR} --python_out=${PROTO_PY_GEN_DIR} ${PROTOS}

clean-skp:
	$(RM) -r $(SKP_DIR)

clean: clean-skp
	$(RM) -r $(BUILD_DIR)
	cargo clean --manifest-path=$(SKI_PASS_DIR)/Cargo.toml --release

gen-nightly:
	python3 ./tools/git-sync-deps
	./bin/gn gen $(BUILD_DIR) --args='is_official_build=false skia_enable_malloc_logging=true'

build-skipass: 
	cargo build --manifest-path=$(SKI_PASS_DIR)/Cargo.toml --release
	cargo install --force cbindgen
	cd ${SKI_PASS_DIR} && cbindgen --config cbindgen.toml --crate ski_pass --output ${SKI_PASS_HEADER}

build-nightly: build-skipass gen-nightly gen-proto
	ninja -C $(BUILD_DIR) skia_opt_membench
	ninja -C $(BUILD_DIR) skia_opt_gen_skps
	ninja -C $(BUILD_DIR) skp_parser
	ninja -C $(BUILD_DIR) skdiff

gen-skps: build-nightly
	mkdir -p $(SKP_DIR)
	$(BUILD_DIR)/skia_opt_gen_skps
	cp ${WEBPAGE_SKPS_DIR}/* ${SKP_DIR}/

local-nightly: clean-skp gen-skps build-nightly
	mkdir -p $(SKP_RENDERS)
	mkdir -p $(SKI_PASS_SKP_RENDERS)
	mkdir -p $(DIFF_REPORT_DIR)
	mkdir -p $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skia_opt_membench --skps $(SKPS) --out_dir $(NIGHTLY_REPORT_DIR)
	$(BUILD_DIR)/skdiff $(SKP_RENDERS) $(SKI_PASS_SKP_RENDERS) $(DIFF_REPORT_DIR)
	$(REPORT_GENERATOR) -d $(NIGHTLY_REPORT_DIR) -t $(REPORT_TEMPLATE)

nightly: clean local-nightly
	scp -r -C $(NIGHTLY_REPORT_DIR) uwplse.org:/var/www/skia/
