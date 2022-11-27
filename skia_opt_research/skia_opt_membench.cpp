#include "include/core/SkBitmap.h"
#include "include/core/SkImageEncoder.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "include/core/SkStream.h"
#include "include/core/SkTime.h"
#include "src/core/SkPicturePriv.h"
#include "src/core/SkRecord.h"
#include "src/core/SkRecordDraw.h"
#include "src/core/SkRecordOpts.h"
#include "src/core/SkRecorder.h"
#include "tools/flags/CommandLineFlags.h"
#include "include/private/SkMalloc.h"

#include "skia_opt_research/ski_pass.pb.h"
#include "skia_opt_research/skia_opt_metrics.pb.h"
#include "skia_opt_research/skp_analyzer.h"

#include <stdio.h>
#include <string.h>
#include <fstream>

#include <google/protobuf/descriptor.h>

static DEFINE_string2(skps, r, "", ".skp files to run the mem bench on.");
static DEFINE_string(out_dir, "", "directory to output .");

static std::map<std::string, int> unsupported_draw_commands_count;

std::string getFileName(const char *filePath) {
    const char *c = strrchr(filePath, '/');
    if (c) {
        return std::string(c+1);
    } else {
        return std::string(filePath);
    }
    abort();
}

void benchmark_skp(
    const char* skpName, 
    skia_opt_metrics::Optimization optType, 
    skia_opt_metrics::SkpBenchmark *benchmark, 
    long long *bytesPerSkp) {

    skia_opt_metrics::OptimizationBenchmark* opt_benchmark = benchmark->add_optimization_benchmark_runs();
    opt_benchmark->set_optimization_type(optType);

    std::unique_ptr<SkStream> stream;

    if (optType == skia_opt_metrics::SKI_PASS) {
        std::string skpOptName(skpName);
        skpOptName += "_opt";
        stream = SkStream::MakeFromFile(skpOptName.c_str());

        std::string skiPassRunInfoProtoFilePath = std::string(skpName);
        skiPassRunInfoProtoFilePath += "_opt.skipass_run.pb";
        std::ifstream skiPassRunInfoIfs(skiPassRunInfoProtoFilePath);
        ski_pass::SkiPassRunInfo run_info;
        run_info.ParseFromIstream(&skiPassRunInfoIfs);

        if (run_info.status() == ski_pass::SkiPassRunStatus::FAILED) {
            fprintf(stderr, "Could not read %s. Skipping this file\n", skpName);
            if (run_info.unsupported_draw_commands().draw_commands().size()) {
                for (auto v : run_info.unsupported_draw_commands().draw_commands()) {
                    unsupported_draw_commands_count[v]++;
                }

            }
            opt_benchmark->set_optimization_status(skia_opt_metrics::OptimizationStatus::FAILED);
            *bytesPerSkp = -1;
            return;
        }
    } else {
        stream = SkStream::MakeFromFile(skpName);
    }

    sk_sp<SkPicture> src(SkPicture::MakeFromStream(stream.get()));
    if (!src) {
        fprintf(stderr, "Could not parse %s into an Skp. Skipping.\n", skpName);
        *bytesPerSkp = -1;
        return;
    }

    const int w = SkScalarCeilToInt(src->cullRect().width());
    const int h = SkScalarCeilToInt(src->cullRect().height());

    SkRecord record;
    SkRecorder recorder(&record, w, h);
    src->playback(&recorder);

    switch (optType) {
        case skia_opt_metrics::NO_OPT:
            break;
        case skia_opt_metrics::SKIA_RECORD_OPTS:
            SkRecordOptimize(&record);
            break;
        case skia_opt_metrics::SKIA_RECORD_OPTS_2:
            SkRecordOptimize2(&record);
            break;
        case skia_opt_metrics::SKI_PASS:
            break;
    }

    SkBitmap bitmap;
    bitmap.allocN32Pixels(w, h);
    SkCanvas canvas(bitmap);

    // There must be a better way to do this.
    std::string outFilePath(FLAGS_out_dir[0]);
    outFilePath += "/" + getFileName(skpName);
    outFilePath += "_" + skia_opt_metrics::Optimization_Name(optType) + "_log.txt";
    fprintf(stdout, "Wrting %s\n", outFilePath.c_str());

    FILE *fp = fopen(outFilePath.c_str(), "w");
    SkpAnalyzer analyzer(&canvas, record.count(), fp);
    for (int i = 0; i < record.count(); i++) {
        record.visit(i, analyzer);
    }
    *bytesPerSkp = analyzer.getTotalMallocBytes();

    opt_benchmark->set_optimization_status(skia_opt_metrics::OptimizationStatus::SUCCESS);
    opt_benchmark->set_malloc_allocated_bytes(*bytesPerSkp);

    if (optType == skia_opt_metrics::NO_OPT) {
        std::string path(FLAGS_out_dir[0]);
        path += "/renders/" + getFileName(skpName) + ".png";
        printf("%s\n", path.c_str());
        SkFILEWStream file(path.c_str());
        SkEncodeImage(&file, bitmap, SkEncodedImageFormat::kPNG, 100);
    }

    if (optType == skia_opt_metrics::SKI_PASS) {
        std::string path(FLAGS_out_dir[0]);
        path += "/skipass_renders/" + getFileName(skpName) + ".png";
        printf("%s\n", path.c_str());
        SkFILEWStream file(path.c_str());
        SkEncodeImage(&file, bitmap, SkEncodedImageFormat::kPNG, 100);
    }

    fclose(fp);
}

int main(int argc, char** argv) {
    GOOGLE_PROTOBUF_VERIFY_VERSION;
    #ifndef SK_MALLOC_LOGGING
        fprintf(stderr, "Compile this program with enable_skia_malloc_logging=true in gn.\n");
        abort();
    #endif

    CommandLineFlags::Parse(argc, argv);

    std::string outFilePath(FLAGS_out_dir[0]);
    outFilePath += "/" + getFileName("000_summary_csv.txt");
    printf("Writing summary to %s\n", outFilePath.c_str());

    FILE *csvSummary = fopen(outFilePath.c_str(), "w");
    fprintf(csvSummary, "skp");

    const google::protobuf::EnumDescriptor *desc = skia_opt_metrics::Optimization_descriptor();
    for (int i=1 /* Skip UNKNOWN */; i < desc->value_count(); i++) {
        fprintf(csvSummary, ",%s", desc->FindValueByNumber(i)->name().c_str());
    }
    fprintf(csvSummary, "\n");


    skia_opt_metrics::SkiaOptBenchmark benchmark = skia_opt_metrics::SkiaOptBenchmark::default_instance();

    for (int i=0; i < FLAGS_skps.count(); i++) {
        skia_opt_metrics::SkpBenchmark *skp_benchmark = benchmark.add_skp_benchmark_runs();
        skp_benchmark->set_skp_name(FLAGS_skps[i]);

        fprintf(csvSummary, "%s,", FLAGS_skps[i]);
        long long bytes_per_skp;

        benchmark_skp(FLAGS_skps[i], skia_opt_metrics::NO_OPT, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        benchmark_skp(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        benchmark_skp(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS_2, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        benchmark_skp(FLAGS_skps[i], skia_opt_metrics::SKI_PASS, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld\n", bytes_per_skp); // Don't put a comma here.
    }
    fclose(csvSummary);

    std::vector< std::pair<int, std::string> > unsupported_draw_commands_sorted;
    for (auto p : unsupported_draw_commands_count) {
        unsupported_draw_commands_sorted.push_back( std::pair<int, std::string>(p.second, p.first));
    }
    sort(unsupported_draw_commands_sorted.begin(), unsupported_draw_commands_sorted.end());
    reverse(unsupported_draw_commands_sorted.begin(), unsupported_draw_commands_sorted.end());

    skia_opt_metrics::SkiPassSummary ski_pass_summary = skia_opt_metrics::SkiPassSummary::default_instance();
    for (auto p : unsupported_draw_commands_sorted) {
        skia_opt_metrics::SkiPassSummary_UnsupportedDrawCommandStats *stats = 
            benchmark.mutable_ski_pass_summary()->add_unsupported_draw_commands();
        stats->set_draw_command(p.second);
        stats->set_count(p.first);
    }

    std::string protoOutFilePath(outFilePath + ".pb");
    std::ofstream protoOut(protoOutFilePath, std::ofstream::out);
    benchmark.SerializePartialToOstream(&protoOut);

    google::protobuf::ShutdownProtobufLibrary();
}
