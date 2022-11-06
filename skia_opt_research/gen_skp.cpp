#include <iostream>
#include "include/core/SkCanvas.h"
#include "include/core/SkData.h"
#include "include/core/SkStream.h"
#include "include/core/SkSurface.h"
#include "tools/flags/CommandLineFlags.h"
#include "tools/trace/EventTracingPriv.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "tools/Resources.h"
#include "tools/debugger/DebugCanvas.h"

using namespace std;

static DEFINE_string(dir, "./skia_opt_research/skps", "directory where to output skps");

void raster(int width, int height, void (*draw)(SkCanvas*), const char* dir, const char *testcase_name) {
    SkPictureRecorder recorder;
    SkCanvas* pictureCanvas = recorder.beginRecording({0, 0, SkScalar(width), SkScalar(height)});
    draw(pictureCanvas);

    sk_sp<SkPicture> picture = recorder.finishRecordingAsPicture();
    sk_sp<SkData> skData = picture->serialize();
    std::string skp_path(dir);
    skp_path.append("/");
    skp_path.append(testcase_name);
    SkFILEWStream skpOut(skp_path.c_str());
    (void)skpOut.write(skData->data(), skData->size());
}

void draw_000_simpleDraw(SkCanvas *canvas) {
    SkPaint paint;
    paint.setColor(SK_ColorRED);
    canvas->drawRect(SkRect::MakeLTRB(20, 20, 100, 100), paint);
}

void draw_001_saveLayerRect(SkCanvas *canvas) {
    SkPaint pRed;
    pRed.setColor(SK_ColorRED);

    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);

    SkPaint pThirtyBlue;
    pThirtyBlue.setColor(SK_ColorBLUE);
    pThirtyBlue.setAlphaf(0.3);


    canvas->drawRect(SkRect::MakeLTRB(10, 70, 60, 120), pRed);
    canvas->drawRect(SkRect::MakeLTRB(150, 70, 200, 120), pRed);

    canvas->saveLayer(nullptr, nullptr);

    canvas->drawRect(SkRect::MakeLTRB(30, 70, 80, 120), pSolidBlue);
    canvas->drawRect(SkRect::MakeLTRB(170, 70, 220, 120), pThirtyBlue);

    canvas->restore();
}

void draw_002_blankSaveLayer(SkCanvas *canvas) {
    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);

    SkPaint pRed;
    pRed.setColor(SK_ColorRED);

    canvas->drawRect(SkRect::MakeLTRB(10, 70, 60, 120), pSolidBlue);

    canvas->saveLayer(nullptr, nullptr);
    canvas->restore();
}

void draw_003_nestedSaveLayer(SkCanvas *canvas) {
    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);
    SkPaint pRed;
    pRed.setColor(SK_ColorRED);

    canvas->saveLayer(nullptr, nullptr);
    canvas->saveLayer(nullptr, nullptr);

    canvas->drawRect(SkRect::MakeLTRB(10, 70, 60, 120), pSolidBlue);
    canvas->drawRect(SkRect::MakeLTRB(170, 70, 220, 120), pRed);

    canvas->restore();
    canvas->restore();
}

void draw_004_drawOval(SkCanvas *canvas) {
    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);
    canvas->drawOval(SkRect::MakeLTRB(10, 70, 60, 120), pSolidBlue);
    canvas->restore();
}


int main(int argc, char **argv) {
    CommandLineFlags::Parse(argc, argv);
    initializeEventTracingForTools();

    raster(512, 512, draw_000_simpleDraw, FLAGS_dir[0], "000_simpleDraw.skp");
    raster(512, 512, draw_001_saveLayerRect, FLAGS_dir[0], "001_saveLayerRect.skp");
    raster(512, 512, draw_002_blankSaveLayer, FLAGS_dir[0], "002_blankSaveLayer.skp");
    raster(512, 512, draw_003_nestedSaveLayer, FLAGS_dir[0], "003_nestedSaveLayer.skp");
    raster(512, 512, draw_004_drawOval, FLAGS_dir[0], "004_drawOval.skp");
}