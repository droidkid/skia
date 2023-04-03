/*
 * Copyright 2014 Google Inc.
 *
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

#include "src/core/SkRecordOpts.h"

#include "include/private/SkTDArray.h"
#include "src/core/SkCanvasPriv.h"
#include "src/core/SkRecordPattern.h"
#include "src/core/SkRecords.h"
#include "src/core/SkRecordDraw.h"

#include "skia_opt_research/ski_pass.pb.h"
#include "skia_opt_research/SkiPass.h"

using namespace SkRecords;

// Most of the optimizations in this file are pattern-based.  These are all defined as structs with:
//   - a Match typedef
//   - a bool onMatch(SkRceord*, Match*, int begin, int end) method,
//     which returns true if it made changes and false if not.

// Run a pattern-based optimization once across the SkRecord, returning true if it made any changes.
// It looks for spans which match Pass::Match, and when found calls onMatch() with that pattern,
// record, and [begin,end) span of the commands that matched.
template <typename Pass>
static bool apply(Pass* pass, SkRecord* record) {
    typename Pass::Match match;
    bool changed = false;
    int begin, end = 0;

    while (match.search(record, &begin, &end)) {
        changed |= pass->onMatch(record, &match, begin, end);
    }
    return changed;
}

///////////////////////////////////////////////////////////////////////////////////////////////////

static void multiple_set_matrices(SkRecord* record) {
    struct {
        typedef Pattern<Is<SetMatrix>,
                        Greedy<Is<NoOp>>,
                        Is<SetMatrix> >
            Match;

        bool onMatch(SkRecord* record, Match* pattern, int begin, int end) {
            record->replace<NoOp>(begin);  // first SetMatrix
            return true;
        }
    } pass;
    while (apply(&pass, record));
}

///////////////////////////////////////////////////////////////////////////////////////////////////

#if 0   // experimental, but needs knowledge of previous matrix to operate correctly
static void apply_matrix_to_draw_params(SkRecord* record) {
    struct {
        typedef Pattern<Is<SetMatrix>,
                        Greedy<Is<NoOp>>,
                        Is<SetMatrix> >
            Pattern;

        bool onMatch(SkRecord* record, Pattern* pattern, int begin, int end) {
            record->replace<NoOp>(begin);  // first SetMatrix
            return true;
        }
    } pass;
    // No need to loop, as we never "open up" opportunities for more of this type of optimization.
    apply(&pass, record);
}
#endif

///////////////////////////////////////////////////////////////////////////////////////////////////

// Turns the logical NoOp Save and Restore in Save-Draw*-Restore patterns into actual NoOps.
struct SaveOnlyDrawsRestoreNooper {
    typedef Pattern<Is<Save>,
                    Greedy<Or<Is<NoOp>, IsDraw>>,
                    Is<Restore>>
        Match;

    bool onMatch(SkRecord* record, Match*, int begin, int end) {
        record->replace<NoOp>(begin);  // Save
        record->replace<NoOp>(end-1);  // Restore
        return true;
    }
};

static bool fold_opacity_layer_color_to_paint(const SkPaint* layerPaint,
                                              bool isSaveLayer,
                                              SkPaint* paint) {
    // We assume layerPaint is always from a saveLayer.  If isSaveLayer is
    // true, we assume paint is too.

    // The alpha folding can proceed if the filter layer paint does not have properties which cause
    // the resulting filter layer to be "blended" in complex ways to the parent layer.
    // TODO: most likely only some xfer modes are the hard constraints
    if (!paint->isSrcOver()) {
        return false;
    }

    if (!isSaveLayer && paint->getImageFilter()) {
        // For normal draws, the paint color is used as one input for the color for the draw. Image
        // filter will operate on the result, and thus we can not change the input.
        // For layer saves, the image filter is applied to the layer contents. The layer is then
        // modulated with the paint color, so it's fine to proceed with the fold for saveLayer
        // paints with image filters.
        return false;
    }

    if (paint->getColorFilter()) {
        // Filter input depends on the paint color.

        // Here we could filter the color if we knew the draw is going to be uniform color.  This
        // should be detectable as drawPath/drawRect/.. without a shader being uniform, while
        // drawBitmap/drawSprite or a shader being non-uniform. However, current matchers don't
        // give the type out easily, so just do not optimize that at the moment.
        return false;
    }

    if (layerPaint) {
        const uint32_t layerColor = layerPaint->getColor();
        // The layer paint color must have only alpha component.
        if (SK_ColorTRANSPARENT != SkColorSetA(layerColor, SK_AlphaTRANSPARENT)) {
            return false;
        }

        // The layer paint can not have any effects.
        if (layerPaint->getPathEffect()  ||
            layerPaint->getShader()      ||
            !layerPaint->isSrcOver()     ||
            layerPaint->getMaskFilter()  ||
            layerPaint->getColorFilter() ||
            layerPaint->getImageFilter()) {
            return false;
        }
        paint->setAlpha(SkMulDiv255Round(paint->getAlpha(), SkColorGetA(layerColor)));
    }

    return true;
}

// Turns logical no-op Save-[non-drawing command]*-Restore patterns into actual no-ops.
struct SaveNoDrawsRestoreNooper {
    // Greedy matches greedily, so we also have to exclude Save and Restore.
    // Nested SaveLayers need to be excluded, or we'll match their Restore!
    typedef Pattern<Is<Save>,
                    Greedy<Not<Or<Is<Save>,
                                  Is<SaveLayer>,
                                  Is<Restore>,
                                  IsDraw>>>,
                    Is<Restore>>
        Match;

    bool onMatch(SkRecord* record, Match*, int begin, int end) {
        // The entire span between Save and Restore (inclusively) does nothing.
        for (int i = begin; i < end; i++) {
            record->replace<NoOp>(i);
        }
        return true;
    }
};
void SkRecordNoopSaveRestores(SkRecord* record) {
    SaveOnlyDrawsRestoreNooper onlyDraws;
    SaveNoDrawsRestoreNooper noDraws;

    // Run until they stop changing things.
    while (apply(&onlyDraws, record) || apply(&noDraws, record));
}

#ifndef SK_BUILD_FOR_ANDROID_FRAMEWORK
static bool effectively_srcover(const SkPaint* paint) {
    if (!paint || paint->isSrcOver()) {
        return true;
    }
    // src-mode with opaque and no effects (which might change opaqueness) is ok too.
    return !paint->getShader() && !paint->getColorFilter() && !paint->getImageFilter() &&
           0xFF == paint->getAlpha() && paint->asBlendMode() == SkBlendMode::kSrc;
}

// For some SaveLayer-[drawing command]-Restore patterns, merge the SaveLayer's alpha into the
// draw, and no-op the SaveLayer and Restore.
struct SaveLayerDrawRestoreNooper {
    typedef Pattern<Is<SaveLayer>, IsDraw, Is<Restore>> Match;

    bool onMatch(SkRecord* record, Match* match, int begin, int end) {
        if (match->first<SaveLayer>()->backdrop) {
            // can't throw away the layer if we have a backdrop
            return false;
        }

        // A SaveLayer's bounds field is just a hint, so we should be free to ignore it.
        SkPaint* layerPaint = match->first<SaveLayer>()->paint;
        SkPaint* drawPaint = match->second<SkPaint>();

        if (nullptr == layerPaint && effectively_srcover(drawPaint)) {
            // There wasn't really any point to this SaveLayer at all.
            return KillSaveLayerAndRestore(record, begin);
        }

        if (drawPaint == nullptr) {
            // We can just give the draw the SaveLayer's paint.
            // TODO(mtklein): figure out how to do this clearly
            return false;
        }

        if (!fold_opacity_layer_color_to_paint(layerPaint, false /*isSaveLayer*/, drawPaint)) {
            return false;
        }

        return KillSaveLayerAndRestore(record, begin);
    }

    static bool KillSaveLayerAndRestore(SkRecord* record, int saveLayerIndex) {
        record->replace<NoOp>(saveLayerIndex);    // SaveLayer
        record->replace<NoOp>(saveLayerIndex+2);  // Restore
        return true;
    }
};
void SkRecordNoopSaveLayerDrawRestores(SkRecord* record) {
    SaveLayerDrawRestoreNooper pass;
    apply(&pass, record);
}
#endif

/* For SVG generated:
  SaveLayer (non-opaque, typically for CSS opacity)
    Save
      ClipRect
      SaveLayer (typically for SVG filter)
      Restore
    Restore
  Restore
*/
struct SvgOpacityAndFilterLayerMergePass {
    typedef Pattern<Is<SaveLayer>, Is<Save>, Is<ClipRect>, Is<SaveLayer>,
                    Is<Restore>, Is<Restore>, Is<Restore>> Match;

    bool onMatch(SkRecord* record, Match* match, int begin, int end) {
        if (match->first<SaveLayer>()->backdrop) {
            // can't throw away the layer if we have a backdrop
            return false;
        }

        SkPaint* opacityPaint = match->first<SaveLayer>()->paint;
        if (nullptr == opacityPaint) {
            // There wasn't really any point to this SaveLayer at all.
            return KillSaveLayerAndRestore(record, begin);
        }

        // This layer typically contains a filter, but this should work for layers with for other
        // purposes too.
        SkPaint* filterLayerPaint = match->fourth<SaveLayer>()->paint;
        if (filterLayerPaint == nullptr) {
            // We can just give the inner SaveLayer the paint of the outer SaveLayer.
            // TODO(mtklein): figure out how to do this clearly
            return false;
        }

        if (!fold_opacity_layer_color_to_paint(opacityPaint, true /*isSaveLayer*/,
                                               filterLayerPaint)) {
            return false;
        }

        return KillSaveLayerAndRestore(record, begin);
    }

    static bool KillSaveLayerAndRestore(SkRecord* record, int saveLayerIndex) {
        record->replace<NoOp>(saveLayerIndex);     // SaveLayer
        record->replace<NoOp>(saveLayerIndex + 6); // Restore
        return true;
    }
};

void SkRecordMergeSvgOpacityAndFilterLayers(SkRecord* record) {
    SvgOpacityAndFilterLayerMergePass pass;
    apply(&pass, record);
}

///////////////////////////////////////////////////////////////////////////////////////////////////

void SkRecordOptimize(SkRecord* record) {
    // This might be useful  as a first pass in the future if we want to weed
    // out junk for other optimization passes.  Right now, nothing needs it,
    // and the bounding box hierarchy will do the work of skipping no-op
    // Save-NoDraw-Restore sequences better than we can here.
    // As there is a known problem with this peephole and drawAnnotation, disable this.
    // If we want to enable this we must first fix this bug:
    //     https://bugs.chromium.org/p/skia/issues/detail?id=5548
//    SkRecordNoopSaveRestores(record);

    // Turn off this optimization completely for Android framework
    // because it makes the following Android CTS test fail:
    // android.uirendering.cts.testclasses.LayerTests#testSaveLayerClippedWithAlpha
#ifndef SK_BUILD_FOR_ANDROID_FRAMEWORK
    SkRecordNoopSaveLayerDrawRestores(record);
#endif
    SkRecordMergeSvgOpacityAndFilterLayers(record);

    record->defrag();
}

void SkRecordOptimize2(SkRecord* record) {
    multiple_set_matrices(record);
    SkRecordNoopSaveRestores(record);
    // See why we turn this off in SkRecordOptimize above.
#ifndef SK_BUILD_FOR_ANDROID_FRAMEWORK
    SkRecordNoopSaveLayerDrawRestores(record);
#endif
    SkRecordMergeSvgOpacityAndFilterLayers(record);

    record->defrag();
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// SKI PASS //
///////////////////////////////////////////////////////////////////////////////////////////////////

/**
  * Given a draw command, extract the paint (if any) onto a ski_pass_proto::SkPaint.
  */
class SkRecordPaintExtractor {
public:
    template <typename T>
    static std::enable_if_t<(T::kTags & kHasPaint_Tag) == kHasPaint_Tag, void>
    fillSkPaintProto(const T& draw, ski_pass_proto::SkPaint* paintPb) {
	    auto paint = AsPtr(draw.paint);
        if (paint != nullptr) {
           SkColor skcolor = paint->getColor();
           ski_pass_proto::SkColor *color = paintPb->mutable_color();
           color->set_alpha_u8(SkColorGetA(skcolor));
           color->set_red_u8(SkColorGetR(skcolor));
           color->set_green_u8(SkColorGetG(skcolor));
           color->set_blue_u8(SkColorGetB(skcolor));

           auto blender = paintPb->mutable_blender();
           auto blendMode = paint->asBlendMode();
           if (blendMode == SkBlendMode::kSrcOver) {
               blender->set_blend_mode(ski_pass_proto::BlendMode::SRC_OVER);
           } 
           else if (blendMode == SkBlendMode::kSrc) {
               blender->set_blend_mode(ski_pass_proto::BlendMode::SRC);
           } 
           else {
               blender->set_blend_mode(ski_pass_proto::BlendMode::UNKNOWN);
           }

           if (paint->getImageFilter() != nullptr) {
               paintPb->mutable_image_filter();
           }
           if (paint->getColorFilter() != nullptr) {
               paintPb->mutable_color_filter();
           }
           if (paint->getPathEffect() != nullptr) {
               paintPb->mutable_path_effect();
           }
           if (paint->getMaskFilter() != nullptr) {
               paintPb->mutable_mask_filter();
           }
           if (paint->getShader() != nullptr) {
               paintPb->mutable_shader();
           }
        }
	    return;
    }



    template <typename T>
    static std::enable_if_t<!(T::kTags & kHasPaint_Tag), void> 
    fillSkPaintProto(const T& draw, ski_pass_proto::SkPaint *paintPb) {
    }

private:
    // Abstracts away whether the paint is always part of the command or optional.
    template <typename T> static const T* AsPtr(const SkRecords::Optional<T>& x) { return x; }
    template <typename T> static const T* AsPtr(const T& x) { return &x; }
};


/**
  * Given a SkRecords, construct it's ski_pass_proto::SkRecord instance (which is the input
  * to the Rust optimizer).
  * Must be called sequentially for all SkRecords in a SkRecord.
  */
class SkiPassRecordBuilder {
    public:
        SkiPassRecordBuilder(ski_pass_proto::SkRecord* skipass_record):
            skipass_record(skipass_record), record_index(0) {}

        template <typename T>
        void operator()(const T& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);
            ski_pass_proto::SkRecords::DrawCommand *draw_command = 
                records->mutable_draw_command();
            draw_command->set_name(std::string(NameOf(command)));
            SkRecordPaintExtractor::fillSkPaintProto(command, draw_command->mutable_paint());
        }

        void operator()(const SkRecords::SaveLayer& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);

            ski_pass_proto::SkRecords_SaveLayer *saveLayer = records->mutable_save_layer();

            if (command.bounds != nullptr) {
                ski_pass_proto::Bounds *bounds = saveLayer->mutable_bounds();
                bounds->set_left(command.bounds->left());
                bounds->set_top(command.bounds->top());
                bounds->set_right(command.bounds->right());
                bounds->set_bottom(command.bounds->bottom());
            }
            SkRecordPaintExtractor::fillSkPaintProto(command, saveLayer->mutable_paint());
            if (command.backdrop != nullptr) {
                saveLayer->mutable_backdrop();
            }
        }

        void operator()(const SkRecords::Concat44& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);

            ski_pass_proto::SkRecords_Concat44 *concat44 = records->mutable_concat44();
            ski_pass_proto::SkM44 *matrix = concat44->mutable_matrix();
            SkScalar v[16];
            command.matrix.getColMajor(v);
            for (int i=0; i < 16; i++) {
                matrix->add_m(v[i]);
            }
        }

        void operator()(const SkRecords::Save& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);
            records->mutable_save();
        }

        void operator()(const SkRecords::Restore& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);
            records->mutable_restore();
        }

        void operator()(const SkRecords::ClipRect& command) {
            ski_pass_proto::SkRecords *records = skipass_record->add_records();
            records->set_index(record_index++);
            auto clip_rect = records->mutable_clip_rect();
            auto bounds = clip_rect->mutable_bounds();

            bounds->set_left(command.rect.left());
            bounds->set_right(command.rect.right());
            bounds->set_top(command.rect.top());
            bounds->set_bottom(command.rect.bottom());

            switch (command.opAA.op()) {
                case(SkClipOp::kDifference):
                    clip_rect->set_clip_op(ski_pass_proto::ClipOp::DIFFERENCE);
                    break;
                case(SkClipOp::kIntersect):
                    clip_rect->set_clip_op(ski_pass_proto::ClipOp::INTERSECT);
                    break;
                default:
                    assert(0);
                    clip_rect->set_clip_op(ski_pass_proto::ClipOp::UNKNOWN_CLIP_OP);
                    break;
            }
            clip_rect->set_do_anti_alias(command.opAA.aa());
        }

        template <typename T>
            static const char* NameOf(const T&) {
#define CASE(U) case SkRecords::U##_Type: return #U;
                switch (T::kType) { SK_RECORD_TYPES(CASE) }
#undef CASE
                SkDEBUGFAIL("Unknown T");
                return "Unknown T";
            }

    private:
        ski_pass_proto::SkRecord* skipass_record;
        int record_index;
};

/**
  * Given a draw command, apply it onto the canvas after modifying the draw command's alpha. 
  */
class SkRecordAlphaApplier {
public:
    SkRecordAlphaApplier(SkCanvas *canvas):
        fDraw(canvas, nullptr, nullptr, 0, nullptr),
        canvas(canvas) {}

    // src/core/SkRecordPattern::IsDraw used as reference for these.
    template <typename T>
    std::enable_if_t<(T::kTags & kHasPaint_Tag) == kHasPaint_Tag,  void>
    operator()(T* draw) {
	    SkPaint *paint = AsPtr(draw->paint);
	    if (paint != nullptr && alpha != 255) {
            paint->setAlpha(this->alpha);
	    }
	    // if paint is nullptr, assume there is nothing to draw.
	    fDraw(*draw);
	    return;
    }

    template <typename T>
    std::enable_if_t<(T::kTags & kHasPaint_Tag) != kHasPaint_Tag, void> operator()(T* draw) {
	    fDraw(*draw);
    }

    void setAlpha(int alpha) {
    	this->alpha = alpha;
    }

private:
    SkRecords::Draw fDraw;
    SkCanvas *canvas;
    int alpha;

    // Abstracts away whether the paint is always part of the command or optional.
    template <typename T> static T* AsPtr(SkRecords::Optional<T>& x) { return x; }
    template <typename T> static T* AsPtr(T& x) { return &x; }
};

/**
  * SkRecord *record: The record to optimize.
  * SkCanvas *canvas: The canvas on which the optimized draw instructions will be applied on.
  * log_fname: File path to dump SkiPass logs
  */
void SkiPassOptimize(SkRecord* record, SkCanvas *canvas, const std::string &log_fname) {
    // Build SkiPassRecord Proto (input to rust optimizer).
    ski_pass_proto::SkRecord skipass_record;
    SkiPassRecordBuilder builder(&skipass_record);
    for (int i=0; i < record->count(); i++) {
        record->visit(i, builder);
    }

    // Serialize and pass the proto onto to the rust optimizer.
    std::string skipass_record_serialized;
    skipass_record.SerializeToString(&skipass_record_serialized);
    SkiPassResultPtr result_ptr = ski_pass_optimize(
            (unsigned char *)skipass_record_serialized.data(), 
            skipass_record_serialized.size());
    std::string result_data((const char *)result_ptr.ptr, result_ptr.len);
    ski_pass_proto::SkiPassRunResult result;
    result.ParseFromString(result_data);

    // Log the results to a file. 
    // TODO: It might be cleaner to let the Rust side handle this.
    FILE *skipass_log = fopen(log_fname.c_str(), "w");
    fprintf(skipass_log, "%s", result.DebugString().c_str());
    fclose(skipass_log);

    SkRecordAlphaApplier alphaApplier(canvas);
    // Apply the instructions passed on by the optimizer and write into SkRecord *record.
    for (auto instruction: result.optimized_program().instructions()) {
        if (instruction.has_copy_record()) {
	        alphaApplier.setAlpha(instruction.copy_record().paint().color().alpha_u8());
            record->mutate((int)(instruction.copy_record().index()), alphaApplier);
        }
        if (instruction.has_save()) {
            canvas->save();
        }
        if (instruction.has_clip_rect()) {
            SkRect rect = SkRect::MakeLTRB(
                instruction.clip_rect().bounds().left(),
                instruction.clip_rect().bounds().top(),
                instruction.clip_rect().bounds().right(),
                instruction.clip_rect().bounds().bottom()
            );

            SkClipOp clipOp;
            switch(instruction.clip_rect().clip_op()) {
                case ski_pass_proto::DIFFERENCE:
                    clipOp = SkClipOp::kDifference;
                    break;
                case ski_pass_proto::INTERSECT:
                case ski_pass_proto::UNKNOWN_CLIP_OP:
                    clipOp = SkClipOp::kIntersect;
                    break;
            }
            bool do_anti_alias = instruction.clip_rect().do_anti_alias();
            canvas->clipRect(rect, clipOp, do_anti_alias);
        }
        if (instruction.has_concat44()) {
            SkScalar v[16];
            for (int i=0; i<16; i++) {
                v[i] = instruction.concat44().matrix().m(i);
            }
            SkM44 skm44 = SkM44::ColMajor(v);
            canvas->concat(skm44);
        }
        if (instruction.has_save_layer()) {
	        SkPaint paint;
	        paint.setARGB(
                instruction.save_layer().paint().color().alpha_u8(),
                instruction.save_layer().paint().color().red_u8(),
                instruction.save_layer().paint().color().green_u8(),
                instruction.save_layer().paint().color().blue_u8()
            );
            if (instruction.save_layer().has_bounds()) {
                canvas->saveLayer(
                        SkRect::MakeLTRB(
                            instruction.save_layer().bounds().left(),
                            instruction.save_layer().bounds().top(),
                            instruction.save_layer().bounds().right(),
                            instruction.save_layer().bounds().bottom()
                        ), 
                        &paint);
            } else {
                canvas->saveLayer(nullptr, &paint);
            }
        }
        if (instruction.has_restore()) {
            canvas->restore();
        }
    }
    free_ski_pass_result(result_ptr);
}
