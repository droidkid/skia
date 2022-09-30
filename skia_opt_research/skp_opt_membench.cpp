
#include "include/core/SkBitmap.h"
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

#include <stdio.h>
#include <string.h>

static DEFINE_string2(skps, r, "", ".skp files to run the mem bench on.");
static DEFINE_string(out_dir, "", "directory to output .");

enum SkOptimizerType {
    NO_OPT,
    SK_RECORD_OPTS,
    SK_RECORD_OPTS2
};

std::string skOptimizerTypeToString(SkOptimizerType optType) {
    switch(optType) {
        case NO_OPT:
            return std::string("no_opt");
            break;
        case SK_RECORD_OPTS:
            return std::string("skRecordOpts");
            break;
        case SK_RECORD_OPTS2:
            return std::string("skRecordOpts2");
            break;
        default:
            abort();
    }
}

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

    static const char* NameOf(const SkRecords::SaveLayer&) {
        return "\x1b[31;1mSaveLayer\x1b[0m";  // Bold red.
    }

    int fDigits;
    int fIndent;
    int fIndex;
    SkRecords::Draw fDraw;
    long long total_malloc_byte_counter;
    FILE *fp;
};

void dump_skp(const char* skpName, sk_sp<SkPicture> src, SkOptimizerType optType, long long *bytesPerSkp) {
    const int w = SkScalarCeilToInt(src->cullRect().width());
    const int h = SkScalarCeilToInt(src->cullRect().height());

    SkRecord record;
    SkRecorder recorder(&record, w, h);
    src->playback(&recorder);

    switch (optType) {
        case NO_OPT:
            break;
        case SK_RECORD_OPTS:
            SkRecordOptimize(&record);
            break;
        case SK_RECORD_OPTS2:
            SkRecordOptimize2(&record);
            break;
    }

    SkBitmap bitmap;
    bitmap.allocN32Pixels(w, h);
    SkCanvas canvas(bitmap);

    // There must be a better way to do this.
    std::string outFilePath(FLAGS_out_dir[0]);
    outFilePath += "/" + getFileName(skpName);
    outFilePath += "_" + skOptimizerTypeToString(optType) + ".log";
    fprintf(stdout, "Wrting %s\n", outFilePath.c_str());

    FILE *fp = fopen(outFilePath.c_str(), "w");
    Dumper dumper(&canvas, record.count(), fp);
    for (int i = 0; i < record.count(); i++) {
        record.visit(i, dumper);
    }
    *bytesPerSkp = dumper.getTotalMallocBytes();
    fclose(fp);
}

int main(int argc, char** argv) {

    #ifndef SK_MALLOC_LOGGING
        fprintf(stderr, "Compile this program with enable_skia_malloc_logging=true in gn.\n");
        abort();
    #endif

    CommandLineFlags::Parse(argc, argv);

    std::string outFilePath(FLAGS_out_dir[0]);
    outFilePath += "/" + getFileName("000_summary.csv");
    printf("Writing summary to %s\n", outFilePath.c_str());

    FILE *csvSummary = fopen(outFilePath.c_str(), "w");
        fprintf(csvSummary, "skp,%s,%s,%s\n", 
            skOptimizerTypeToString(NO_OPT).c_str(),
            skOptimizerTypeToString(SK_RECORD_OPTS).c_str(),
            skOptimizerTypeToString(SK_RECORD_OPTS2).c_str()
        );


    for (int i=0; i < FLAGS_skps.count(); i++) {
        std::unique_ptr<SkStream> stream = SkStream::MakeFromFile(FLAGS_skps[i]);
        if (!stream) {
            fprintf(stderr, "Could not read %s. Skipping this file\n", FLAGS_skps[i]);
            continue;
        }
        sk_sp<SkPicture> src(SkPicture::MakeFromStream(stream.get()));
        if (!src) {
            fprintf(stderr, "Could not parse %s into an Skp. Skipping.\n", FLAGS_skps[i]);
            continue;
        }


        fprintf(csvSummary, "%s,", FLAGS_skps[i]);
        long long bytes_per_skp;

        dump_skp(FLAGS_skps[i], src, NO_OPT, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        dump_skp(FLAGS_skps[i], src, SK_RECORD_OPTS, &bytes_per_skp);
        fprintf(csvSummary, "%lld,", bytes_per_skp);
        dump_skp(FLAGS_skps[i], src, SK_RECORD_OPTS2,&bytes_per_skp);
        // DON'T PUT A COMMA FOR THE LAST VALUE.
        fprintf(csvSummary, "%lld\n", bytes_per_skp);

    }
    fclose(csvSummary);
}