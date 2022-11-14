
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

class Dumper {
public:
    explicit Dumper(SkCanvas* canvas, int count, FILE *fp)
        : fDigits(0)
        , fIndent(0)
        , fIndex(0)
        , fDraw(canvas, nullptr, nullptr, 0, nullptr)
        , total_malloc_byte_counter(0)
        , fp(fp)
    {
        while (count > 0) {
            count /= 10;
            fDigits++;
        }
    }

    template <typename T>
    void operator()(const T& command) {
        auto start = SkTime::GetNSecs();

        // This comes from private/SkMalloc.h
        malloc_byte_accumlator = 0;
        fDraw(command);
        total_malloc_byte_counter += malloc_byte_accumlator;

        this->print(command, SkTime::GetNSecs() - start, malloc_byte_accumlator);
    }

    void operator()(const SkRecords::NoOp&) {
        // Move on without printing anything.
    }

    template <typename T>
    void print(const T& command, double ns, long long bytes) {
        this->printNameAndTimeAndBytes(command, ns, bytes);
    }

    void print(const SkRecords::Restore& command, double ns, long long bytes) {
        --fIndent;
        this->printNameAndTimeAndBytes(command, ns, bytes);
    }

    void print(const SkRecords::Save& command, double ns, long long bytes) {
        this->printNameAndTimeAndBytes(command, ns, bytes);
        ++fIndent;
    }

    void print(const SkRecords::SaveLayer& command, double ns, long long bytes) {
        this->printNameAndTimeAndBytes(command, ns, bytes);
        ++fIndent;
    }

    void print(const SkRecords::DrawPicture& command, double ns, long long bytes) {
        this->printNameAndTimeAndBytes(command, ns, bytes);

        if (auto bp = SkPicturePriv::AsSkBigPicture(command.picture)) {
            ++fIndent;

            const SkRecord& record = *bp->record();
            for (int i = 0; i < record.count(); i++) {
                record.visit(i, *this);
            }

            --fIndent;
        }
    }

    void print(const SkRecords::DrawAnnotation& command, double ns, long long bytes) {
        int us = (int)(ns * 1e-3);
        fprintf(fp, "%10lldB ", bytes);
        fprintf(fp, "%*d ", fDigits, fIndex++);
        for (int i = 0; i < fIndent; i++) {
            fprintf(fp, "    ");
        }
        fprintf(fp, "%6dus  ", us);
        fprintf(fp, "DrawAnnotation [%g %g %g %g] %s\n",
               command.rect.left(), command.rect.top(), command.rect.right(), command.rect.bottom(),
               command.key.c_str());
    }

    void finish() {
        fclose(fp);
    }

    long long getTotalMallocBytes() {
        return total_malloc_byte_counter;
    }
private:
    template <typename T>
    void printNameAndTimeAndBytes(const T& command, double ns, long long bytes) {
        int us = (int)(ns * 1e-3);
        fprintf(fp, "%10lldB ", bytes);
        fprintf(fp, "%*d ", fDigits, fIndex++);
        for (int i = 0; i < fIndent; i++) {
            fprintf(fp, "    ");
        }
        fprintf(fp, "%6dus  ", us);
        fprintf(fp, "%s\n", NameOf(command));
        // puts(NameOf(command));
    }

    template <typename T>
    static const char* NameOf(const T&) {
    #define CASE(U) case SkRecords::U##_Type: return #U;
        switch (T::kType) { SK_RECORD_TYPES(CASE) }
    #undef CASE
        SkDEBUGFAIL("Unknown T");
        return "Unknown T";
    }

    int fDigits;
    int fIndent;
    int fIndex;
    SkRecords::Draw fDraw;
    long long total_malloc_byte_counter;
    FILE *fp;
};

void dump_skp(
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
                    fprintf(stderr, "Unsupported Draw Command: %s.", v.c_str());
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
    Dumper dumper(&canvas, record.count(), fp);
    for (int i = 0; i < record.count(); i++) {
        record.visit(i, dumper);
    }
    *bytesPerSkp = dumper.getTotalMallocBytes();

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

        dump_skp(FLAGS_skps[i], skia_opt_metrics::NO_OPT, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        dump_skp(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        dump_skp(FLAGS_skps[i], skia_opt_metrics::SKIA_RECORD_OPTS_2, skp_benchmark, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        dump_skp(FLAGS_skps[i], skia_opt_metrics::SKI_PASS, skp_benchmark, &bytes_per_skp);
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
