#include <iostream>
#include "include/core/SkCanvas.h"
#include "include/core/SkFont.h"
#include "include/core/SkImageFilter.h"
#include "include/effects/SkImageFilters.h"
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

void draw_005_clipRect(SkCanvas *canvas) {
    SkPaint paint;
    canvas->drawOval(SkRect::MakeLTRB(10, 0, 260, 120), paint);
  	canvas->save();
    canvas->clipRect(SkRect::MakeWH(90, 80));
  	    canvas->save();
        canvas->clipRect(SkRect::MakeWH(90, 80));
        canvas->drawOval(SkRect::MakeLTRB(40, 0, 160, 120), paint);
        canvas->restore();
    canvas->drawOval(SkRect::MakeLTRB(40, 0, 160, 120), paint);
    canvas->restore();
}

void draw_006_clipRect2(SkCanvas *canvas) {
    SkPaint paint;

    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);

    canvas->saveLayer(nullptr, nullptr);
    canvas->clipRect(SkRect::MakeWH(90, 80));
    canvas->drawCircle(100, 100, 60, paint);
    canvas->restore();

    canvas->drawRect(SkRect::MakeLTRB(90, 90, 110, 130), pSolidBlue);
}

void draw_007_saveLayer(SkCanvas *canvas) {
    SkPaint pRed;
    pRed.setColor(SK_ColorRED);

    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);

    SkPaint pThirtyBlue;
    pThirtyBlue.setColor(SK_ColorBLUE);
    pThirtyBlue.setAlphaf(0.3);

    SkPaint alpha;
    alpha.setAlphaf(0.3);

    // First row: Draw two opaque red rectangles into the 0th layer. Then draw two blue
    // rectangles overlapping the red, one is solid, the other is 30% transparent.
    canvas->drawRect(SkRect::MakeLTRB(10, 10, 60, 60), pRed);
    canvas->drawRect(SkRect::MakeLTRB(150, 10, 200, 60), pRed);

    canvas->drawRect(SkRect::MakeLTRB(30, 10, 80, 60), pSolidBlue);
    canvas->drawRect(SkRect::MakeLTRB(170, 10, 220, 60), pThirtyBlue);

    // Second row: Draw two opaque red rectangles into the 0th layer. Then save a new layer;
    // when the 1st layer gets merged onto the 0th layer (i.e. when restore() is called), it will
    // use the provided paint to do so. In this case, the paint is set to have 30% opacity, but
    // it could also have things set like blend modes or image filters.
    canvas->drawRect(SkRect::MakeLTRB(10, 70, 60, 120), pRed);
    canvas->drawRect(SkRect::MakeLTRB(150, 70, 200, 120), pRed);

    canvas->saveLayer(nullptr, &alpha);

    // In the 1st layer, draw the same blue overlapping rectangles as in the first row. Notice in
    // the final output, we have two different shades of purple. The layer's alpha made the
    // opaque blue rectangle transparent, and it made the transparent blue rectangle even more so
    canvas->drawRect(SkRect::MakeLTRB(30, 70, 80, 120), pSolidBlue);
    canvas->drawRect(SkRect::MakeLTRB(170, 70, 220, 120), pThirtyBlue);

    canvas->restore();

    // Third row: save the layer first, before drawing the two red rectangle, followed by the
    // overlapping blue rectangles. Notice that the blue overwrites the red in the same way as
    // the first row because the alpha of the layer is not applied until the layer is restored.
    canvas->saveLayer(nullptr, &alpha);

    canvas->drawRect(SkRect::MakeLTRB(10, 130, 60, 180), pRed);
    canvas->drawRect(SkRect::MakeLTRB(150, 130, 200, 180), pRed);

    canvas->drawRect(SkRect::MakeLTRB(30, 130, 80, 180), pSolidBlue);
    canvas->drawRect(SkRect::MakeLTRB(170, 130, 220, 180), pThirtyBlue);

    canvas->restore();
}

void draw_008_noOpSaveLayerRemove(SkCanvas *canvas) {
    SkPaint pSolidBlue;
    pSolidBlue.setColor(SK_ColorBLUE);

    // SkRecordOpts optimizes this...
    canvas->saveLayer(nullptr, nullptr);
    canvas->drawRect(SkRect::MakeLTRB(90, 90, 110, 130), pSolidBlue);
    canvas->restore();

    // ...but not this!!??
    canvas->saveLayer(nullptr, nullptr);
    canvas->restore();

    SkPaint alpha;
    alpha.setAlphaf(0.3);
    canvas->saveLayer(nullptr, &alpha);
    canvas->drawRect(SkRect::MakeLTRB(190, 190, 110, 130), pSolidBlue);
    canvas->restore();
}

void draw_009_recordOptsTest_SingleNoopSaveRestore(SkCanvas *canvas) {
    // This is effectively a NoOp. 
    canvas->save();
    canvas->clipRect(SkRect::MakeWH(200, 200));
    canvas->restore();
}

void draw_010_recordOptsTest_NoopSaveRestores(SkCanvas *canvas) {
    canvas->save();

        canvas->save();
        canvas->restore();

        // This is a noOp. 
        canvas->save();
            canvas->clipRect(SkRect::MakeWH(200, 200));
            canvas->clipRect(SkRect::MakeWH(100, 100));
        canvas->restore();

    canvas->restore();
}

void draw_011_recordOptsTest_NoopSaveLayerDrawRestore(SkCanvas *canvas) {
	// Copied from RecordOptsTest.cpp
    SkRect bounds = SkRect::MakeWH(100, 200);
    SkRect   draw = SkRect::MakeWH(50, 60);

    SkPaint alphaOnlyLayerPaint, translucentLayerPaint, xfermodeLayerPaint;
    alphaOnlyLayerPaint.setColor(0x03000000);  // Only alpha.
    translucentLayerPaint.setColor(0x03040506);  // Not only alpha.
    xfermodeLayerPaint.setBlendMode(SkBlendMode::kDstIn);  // Any effect will do.

    SkPaint opaqueDrawPaint, translucentDrawPaint;
    opaqueDrawPaint.setColor(0xFF020202);  // Opaque.
    translucentDrawPaint.setColor(0x0F020202);  // Not opaque.

    // Can be killed.
    canvas->saveLayer(nullptr, nullptr);
        canvas->drawRect(draw, opaqueDrawPaint);
    canvas->restore();

    // Can be killed.
    canvas->saveLayer(&bounds, nullptr);
        canvas->drawRect(draw, opaqueDrawPaint);
    canvas->restore();

    // Should NOT BE killed! See NotOnlyAlphaPaintSaveLayer case.
    canvas->saveLayer(nullptr, &translucentLayerPaint);
        canvas->drawRect(draw, opaqueDrawPaint);
    canvas->restore();

    // Should NOT BE killed!
    canvas->saveLayer(nullptr, &xfermodeLayerPaint);
        canvas->drawRect(draw, opaqueDrawPaint);
    canvas->restore();

    // Can be killed.
    // SaveLayer/Restore removed: we can fold in the alpha!
    canvas->saveLayer(nullptr, &alphaOnlyLayerPaint);
        canvas->drawRect(draw, translucentDrawPaint);
    canvas->restore();

    // Can be killed.
    // SaveLayer/Restore removed: we can fold in the alpha!
    canvas->saveLayer(nullptr, &alphaOnlyLayerPaint);
        canvas->drawRect(draw, opaqueDrawPaint);
    canvas->restore();
}

void draw_012_recordOptsTest_NotOnlyAlphaPaintSaveLayer(SkCanvas *canvas) {
	// Copied from RecordOptsTest.cpp
    SkRect   draw1 = SkRect::MakeWH(50, 60);
    SkRect   draw2 = SkRect::MakeWH(150, 60);


    SkPaint translucentLayerPaint;
    translucentLayerPaint.setColor(0x80808080);  // Not only alpha.

    SkPaint opaqueDrawPaint2;
    opaqueDrawPaint2.setColor(0xFF800000);  // Opaque.
                                           //
    SkPaint opaqueDrawPaint1;
    opaqueDrawPaint1.setColor(0xFF102030);  // Opaque.

    canvas->drawRect(draw1, opaqueDrawPaint1);
    // Can NOT be killed, you get a diff.
    canvas->saveLayer(nullptr, &translucentLayerPaint);
        canvas->drawRect(draw2, opaqueDrawPaint2);
    canvas->restore();
}

void draw_013_captureSaveLayerState_scaleOutside(SkCanvas *canvas) {
    SkPaint paint;
    paint.setColor(SkColorSetRGB(255, 0, 0));
    SkFont font(nullptr, 80);
    font.setScaleX(.3f);

  	SkPaint lPaint;
  	sk_sp<SkImageFilter> shadowFilter = SkImageFilters::DropShadow(
             5.0f, 0.0f, 5.0f, 0.0f, SK_ColorBLUE, nullptr);
	lPaint.setImageFilter(shadowFilter);
    SkRect rect[1] = {{ 10, 20, 90, 110 }};

    canvas->scale(2.0, 2.0);
  	    canvas->saveLayer(nullptr, &lPaint);
        canvas->drawString("Hello", rect[0].fLeft + 10, rect[0].fBottom - 10, font, paint);
    canvas->restore();
}

void draw_014_captureSaveLayerState_scaleInside(SkCanvas *canvas) {
    SkPaint paint;
    paint.setColor(SkColorSetRGB(255, 0, 0));
    SkFont font(nullptr, 80);
    font.setScaleX(.3f);

  	SkPaint lPaint;
  	sk_sp<SkImageFilter> shadowFilter = SkImageFilters::DropShadow(
             5.0f, 0.0f, 5.0f, 0.0f, SK_ColorBLUE, nullptr);
	lPaint.setImageFilter(shadowFilter);
    SkRect rect[1] = {{ 10, 20, 90, 110 }};

  	canvas->saveLayer(nullptr, &lPaint);
        canvas->scale(2.0, 2.0);
        canvas->drawString("Hello", rect[0].fLeft + 10, rect[0].fBottom - 10, font, paint);
    canvas->restore();
}

void draw_015_mergeSrcOverTree(SkCanvas *canvas) {
    SkPaint red;
    red.setColor(SK_ColorRED);
  	red.setAlphaf(0.5);  
  
  	SkPaint blue;
    blue.setColor(SK_ColorBLUE);
  	blue.setAlphaf(0.5);  

    SkPaint green;
    green.setColor(SK_ColorGREEN);
  	green.setAlphaf(0.5);  

    SkPaint yellow;
    yellow.setColor(SK_ColorYELLOW);
  	yellow.setAlphaf(0.5);  

  
  	canvas->drawRect(SkRect::MakeLTRB(10, 60, 100, 120), red);
  	canvas->saveLayer(nullptr, nullptr);
  		canvas->drawRect(SkRect::MakeLTRB(50, 60, 120, 120), blue);
  		canvas->saveLayer(nullptr, nullptr);
  			canvas->drawRect(SkRect::MakeLTRB(30, 30, 90, 100), green);
        	canvas->drawRect(SkRect::MakeLTRB(30, 110, 90, 140), yellow);
  		canvas->restore();
    canvas->restore();
}


void draw_016_collapseInnerMerge(SkCanvas *canvas) {
    SkPaint red;
    red.setColor(SK_ColorRED);
  	red.setAlphaf(0.5);  
  
  	SkPaint blue;
    blue.setColor(SK_ColorBLUE);
  	blue.setAlphaf(0.5);  

    SkPaint green;
    green.setColor(SK_ColorGREEN);
  	green.setAlphaf(0.5);  

    SkPaint yellow;
    yellow.setColor(SK_ColorYELLOW);
  	yellow.setAlphaf(0.5);  
  
  	canvas->drawRect(SkRect::MakeLTRB(10, 60, 100, 120), red);
  	canvas->saveLayer(nullptr, nullptr);
  		canvas->drawRect(SkRect::MakeLTRB(50, 60, 120, 120), blue);
  		canvas->saveLayer(nullptr, nullptr);
  			canvas->drawRect(SkRect::MakeLTRB(30, 30, 90, 100), green);
  		canvas->restore();
    canvas->restore();
}

void draw_017_TestClipRectIntersection(SkCanvas *canvas) {
    SkPaint p;
    p.setColor(SK_ColorRED);
    p.setAntiAlias(true);

    canvas->clipRect(SkRect::MakeLTRB(30, 30, 200, 200));
    canvas->clipRect(SkRect::MakeLTRB(0, 0, 35, 35));
    canvas->drawRect(SkRect::MakeLTRB(10, 10, 500, 500), p);
}

int main(int argc, char **argv) {
    CommandLineFlags::Parse(argc, argv);
    initializeEventTracingForTools();

    raster(512, 512, draw_000_simpleDraw, FLAGS_dir[0], "000_simpleDraw.skp");
    raster(512, 512, draw_001_saveLayerRect, FLAGS_dir[0], "001_saveLayerRect.skp");
    raster(512, 512, draw_002_blankSaveLayer, FLAGS_dir[0], "002_blankSaveLayer.skp");
    raster(512, 512, draw_003_nestedSaveLayer, FLAGS_dir[0], "003_nestedSaveLayer.skp");
    raster(512, 512, draw_004_drawOval, FLAGS_dir[0], "004_drawOval.skp");
    raster(512, 512, draw_005_clipRect, FLAGS_dir[0], "005_clipRect.skp");
    raster(512, 512, draw_006_clipRect2, FLAGS_dir[0], "006_clipRect2.skp");
    raster(512, 512, draw_007_saveLayer, FLAGS_dir[0], "007_saveLayer.skp");
    raster(512, 512, draw_008_noOpSaveLayerRemove, FLAGS_dir[0], "008_noOpSave.skp");

    // Some tests from RecordOptsTest.cpp
    raster(512, 512, draw_009_recordOptsTest_SingleNoopSaveRestore, FLAGS_dir[0], "SingleNoopSaveRestore.skp");
    raster(512, 512, draw_010_recordOptsTest_NoopSaveRestores, FLAGS_dir[0], "NoopSaveRestores.skp");
    raster(512, 512, draw_011_recordOptsTest_NoopSaveLayerDrawRestore, FLAGS_dir[0], "NoopSaveLayerDrawRestore.skp");
    raster(512, 512, draw_012_recordOptsTest_NotOnlyAlphaPaintSaveLayer, FLAGS_dir[0], "NotOnlyAlphaPaintSaveLayer.skp");

    raster(512, 512, draw_013_captureSaveLayerState_scaleOutside, FLAGS_dir[0], "013_captureSaveLayerState_scaleOutside.skp");
    raster(512, 512, draw_014_captureSaveLayerState_scaleInside, FLAGS_dir[0], "014_captureSaveLayerState_scaleInside.skp");
    raster(512, 512, draw_015_mergeSrcOverTree, FLAGS_dir[0], "015_mergeSrcOverTree.skp");
    raster(512, 512, draw_016_collapseInnerMerge, FLAGS_dir[0], "016_collapseInnerMerge.skp");
    raster(512, 512, draw_017_TestClipRectIntersection, FLAGS_dir[0], "draw_017_TestClipRectIntersection");
}
