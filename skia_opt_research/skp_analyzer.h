/*
 * Given a Skp, prints out all the SkRecords in that Skp, along with
 * the time and memory (measured in SkMalloc) that each SkRecord took.
 *
 * WARNING: This memory measurement is simplistic, and we expect that the counter
 * in SkMalloc.h is reset before this is called.
 * This header is also not reusable, it's expected to be part of skia_opt_membench.cpp
 */

class SkpAnalyzer {
public:
    explicit SkpAnalyzer(SkCanvas* canvas, int count, FILE *fp)
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

