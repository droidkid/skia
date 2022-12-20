#include "include/core/SkBitmap.h"
#include "include/core/SkImageEncoder.h"
#include "include/core/SkEncodedImageFormat.h"
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

void benchmark_optimization(
        const char* skpName, 
        skia_opt_metrics::Optimization optType, 
        skia_opt_metrics::OptimizationBenchmark *benchmark) {

    std::string outDir(FLAGS_out_dir[0]);
    benchmark->set_optimization_type(optType);

    // Get SKP from file.
    std::unique_ptr<SkStream> stream;
    stream = SkStream::MakeFromFile(skpName);
    sk_sp<SkPicture> src(SkPicture::MakeFromStream(stream.get()));
    if (!src) {
        benchmark->set_optimization_status(skia_opt_metrics::OptimizationStatus::FAILED);
        fprintf(stderr, "Error loading %s Skp. Skipping.\n", skpName);
        return;
    }

    // Load the SKP into a SkRecord.
    const int w = SkScalarCeilToInt(src->cullRect().width());
    const int h = SkScalarCeilToInt(src->cullRect().height());
    SkRecord skp_record;
    SkRecorder skp_recorder(&skp_record, w, h);
    src->playback(&skp_recorder);

    SkRecord skipass_record;
    SkRecorder skipass_recorder(&skipass_record, w, h);

    SkRecord *record = &skp_record;

    // Optimize SkRecord.
    switch (optType) {
        case skia_opt_metrics::NO_OPT:
            break;
        case skia_opt_metrics::SKIA_RECORD_OPTS:
            SkRecordOptimize(record);
            break;
        case skia_opt_metrics::SKIA_RECORD_OPTS_2:
            SkRecordOptimize2(record);
            break;
        case skia_opt_metrics::SKI_PASS:
            SkiPassOptimize(record, &skipass_recorder);
            record = &skipass_record;
            break;
    }

    // Create a Canvas.
    SkBitmap bitmap;
    bitmap.allocN32Pixels(w, h);
    SkCanvas canvas(bitmap);

    // Record the analysis onto a canvas and log file.
    std::string optimization_log_fname = 
        outDir + "/" +
        getFileName(skpName) + "_" + 
        skia_opt_metrics::Optimization_Name(optType) + 
        "_log.txt";
    FILE *fp = fopen(optimization_log_fname.c_str(), "w");
    SkpAnalyzer analyzer(&canvas, record->count(), fp);
    for (int i = 0; i < record->count(); i++) {
        record->visit(i, analyzer);
    }
    fclose(fp);

    // Copy the benchmarks into the proto.
    benchmark->set_optimization_status(skia_opt_metrics::OptimizationStatus::SUCCESS);
    benchmark->set_malloc_allocated_bytes(analyzer.getTotalMallocBytes());

    // Render NO_OPT image for Diffing.
    if (optType == skia_opt_metrics::NO_OPT) {
        std::string path = 
            outDir + 
            "/renders/" + 
            getFileName(skpName) + ".png";
        SkFILEWStream file(path.c_str());
        SkEncodeImage(&file, bitmap, SkEncodedImageFormat::kPNG, 100);
    }

    // Render SKI_PASS image for Diffing.
    if (optType == skia_opt_metrics::SKI_PASS) {
        std::string path = 
            outDir + 
            "/skipass_renders/" + 
            getFileName(skpName) + ".png";
        SkFILEWStream file(path.c_str());
        SkEncodeImage(&file, bitmap, SkEncodedImageFormat::kPNG, 100);

        std::string skp_path = 
            outDir + 
            "/" + 
            getFileName(skpName) + ".skp";

        SkPictureRecorder recorder;
        SkCanvas* skpCanvas = recorder.beginRecording({0, 0, SkScalar(w), SkScalar(h)});
        FILE *fp2 = fopen("/dev/null", "w");
        SkpAnalyzer analyzer2(skpCanvas, record->count(), fp2);
        for (int i = 0; i < record->count(); i++) {
            record->visit(i, analyzer2);
        }
        fclose(fp2);
        sk_sp<SkPicture> picture = recorder.finishRecordingAsPicture();
        sk_sp<SkData> skData = picture->serialize();
        SkFILEWStream skpOut(skp_path.c_str());
        (void)skpOut.write(skData->data(), skData->size());
    }
}

int main(int argc, char** argv) {
    GOOGLE_PROTOBUF_VERIFY_VERSION;
#ifndef SK_MALLOC_LOGGING
    fprintf(stderr, "Compile this program with enable_skia_malloc_logging=true in gn.\n");
    abort();
#endif

    CommandLineFlags::Parse(argc, argv);
    skia_opt_metrics::SkiaOptBenchmark benchmark = skia_opt_metrics::SkiaOptBenchmark::default_instance();
    std::string outFilePath(FLAGS_out_dir[0]);

    for (int i=0; i < FLAGS_skps.size(); i++) {
        skia_opt_metrics::SkpBenchmark *skp_benchmark = benchmark.add_skp_benchmark_runs();

        skp_benchmark->set_skp_name(FLAGS_skps[i]);

        // TODO: Put this in a for loop.
        benchmark_optimization(FLAGS_skps[i], skia_opt_metrics::NO_OPT, skp_benchmark->add_optimization_benchmark_runs());
        benchmark_optimization(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS, skp_benchmark->add_optimization_benchmark_runs());
        benchmark_optimization(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS_2, skp_benchmark->add_optimization_benchmark_runs());
        benchmark_optimization(FLAGS_skps[i], skia_opt_metrics::SKI_PASS, skp_benchmark->add_optimization_benchmark_runs());
    }

    std::string protoOutFilePath(outFilePath + "/benchmark.pb");
    std::ofstream protoOut(protoOutFilePath, std::ofstream::out);
    benchmark.SerializeToOstream(&protoOut);

    google::protobuf::ShutdownProtobufLibrary();
}
