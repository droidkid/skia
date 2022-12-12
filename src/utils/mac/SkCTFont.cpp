/*
 * Copyright 2020 Google Inc.
 *
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

#include "include/core/SkTypes.h"

#if defined(SK_BUILD_FOR_MAC) || defined(SK_BUILD_FOR_IOS)

#include "include/core/SkData.h"
#include "include/core/SkRefCnt.h"
#include "include/private/SkOnce.h"
#include "src/sfnt/SkOTTable_OS_2.h"
#include "src/sfnt/SkSFNTHeader.h"
#include "src/utils/mac/SkCTFont.h"
#include "src/utils/mac/SkUniqueCFRef.h"

#ifdef SK_BUILD_FOR_MAC
#import <ApplicationServices/ApplicationServices.h>
#endif

#ifdef SK_BUILD_FOR_IOS
#include <CoreText/CoreText.h>
#include <CoreText/CTFontManager.h>
#include <CoreGraphics/CoreGraphics.h>
#include <CoreFoundation/CoreFoundation.h>
#endif

#include <dlfcn.h>

static constexpr CGBitmapInfo kBitmapInfoRGB = ((CGBitmapInfo)kCGImageAlphaNoneSkipFirst |
                                                kCGBitmapByteOrder32Host);

/** Drawn in FontForge, reduced with fonttools ttx, converted by xxd -i,
 *  this TrueType font contains a glyph of the spider.
 *
 *  To re-forge the original bytes of the TrueType font file,
 *  remove all ',|( +0x)' from this definition,
 *  copy the data to the clipboard,
 *  run 'pbpaste | xxd -p -r - spider.ttf'.
 */
static constexpr const uint8_t kSpiderSymbol_ttf[] = {
    0x00, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x80, 0x00, 0x03, 0x00, 0x40,
    0x47, 0x44, 0x45, 0x46, 0x00, 0x14, 0x00, 0x14, 0x00, 0x00, 0x07, 0xa8,
    0x00, 0x00, 0x00, 0x18, 0x4f, 0x53, 0x2f, 0x32, 0x8a, 0xf4, 0xfb, 0xdb,
    0x00, 0x00, 0x01, 0x48, 0x00, 0x00, 0x00, 0x60, 0x63, 0x6d, 0x61, 0x70,
    0xe0, 0x7f, 0x10, 0x7e, 0x00, 0x00, 0x01, 0xb8, 0x00, 0x00, 0x00, 0x54,
    0x67, 0x61, 0x73, 0x70, 0xff, 0xff, 0x00, 0x03, 0x00, 0x00, 0x07, 0xa0,
    0x00, 0x00, 0x00, 0x08, 0x67, 0x6c, 0x79, 0x66, 0x97, 0x0b, 0x6a, 0xf6,
    0x00, 0x00, 0x02, 0x18, 0x00, 0x00, 0x03, 0x40, 0x68, 0x65, 0x61, 0x64,
    0x0f, 0xa2, 0x24, 0x1a, 0x00, 0x00, 0x00, 0xcc, 0x00, 0x00, 0x00, 0x36,
    0x68, 0x68, 0x65, 0x61, 0x0e, 0xd3, 0x07, 0x3f, 0x00, 0x00, 0x01, 0x04,
    0x00, 0x00, 0x00, 0x24, 0x68, 0x6d, 0x74, 0x78, 0x10, 0x03, 0x00, 0x44,
    0x00, 0x00, 0x01, 0xa8, 0x00, 0x00, 0x00, 0x0e, 0x6c, 0x6f, 0x63, 0x61,
    0x01, 0xb4, 0x00, 0x28, 0x00, 0x00, 0x02, 0x0c, 0x00, 0x00, 0x00, 0x0a,
    0x6d, 0x61, 0x78, 0x70, 0x00, 0x4a, 0x01, 0x4d, 0x00, 0x00, 0x01, 0x28,
    0x00, 0x00, 0x00, 0x20, 0x6e, 0x61, 0x6d, 0x65, 0xc3, 0xe5, 0x39, 0xd4,
    0x00, 0x00, 0x05, 0x58, 0x00, 0x00, 0x02, 0x28, 0x70, 0x6f, 0x73, 0x74,
    0xff, 0x03, 0x00, 0x67, 0x00, 0x00, 0x07, 0x80, 0x00, 0x00, 0x00, 0x20,
    0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0b, 0x0f, 0x08, 0x1d,
    0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x0b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xd1, 0x97, 0xa8, 0x5a, 0x00, 0x00, 0x00, 0x00, 0xd6, 0xe8, 0x32, 0x33,
    0x00, 0x03, 0xff, 0x3b, 0x08, 0x00, 0x05, 0x55, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x55, 0xff, 0x3b, 0x01, 0x79, 0x08, 0x00, 0x00, 0x03, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x04, 0x01, 0x1c, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x2e,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x08, 0x00, 0x01, 0x90, 0x00, 0x05,
    0x00, 0x00, 0x05, 0x33, 0x05, 0x99, 0x00, 0x00, 0x01, 0x1e, 0x05, 0x33,
    0x05, 0x99, 0x00, 0x00, 0x03, 0xd7, 0x00, 0x66, 0x02, 0x12, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x73, 0x6b, 0x69, 0x61, 0x00, 0xc0, 0x00, 0x00, 0xf0, 0x21,
    0x06, 0x66, 0xfe, 0x66, 0x01, 0x79, 0x05, 0x55, 0x00, 0xc5, 0x80, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x01, 0x08, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x04, 0x00, 0x48,
    0x00, 0x00, 0x00, 0x0e, 0x00, 0x08, 0x00, 0x02, 0x00, 0x06, 0x00, 0x00,
    0x00, 0x09, 0x00, 0x0d, 0x00, 0x1d, 0x00, 0x21, 0xf0, 0x21, 0xff, 0xff,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0d, 0x00, 0x1d, 0x00, 0x21,
    0xf0, 0x21, 0xff, 0xff, 0x00, 0x01, 0xff, 0xf9, 0xff, 0xf5, 0xff, 0xe4,
    0xff, 0xe2, 0x0f, 0xe2, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14,
    0x00, 0x14, 0x00, 0x14, 0x01, 0xa0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x44,
    0x00, 0x00, 0x02, 0x64, 0x05, 0x55, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00,
    0x33, 0x11, 0x21, 0x11, 0x25, 0x21, 0x11, 0x21, 0x44, 0x02, 0x20, 0xfe,
    0x24, 0x01, 0x98, 0xfe, 0x68, 0x05, 0x55, 0xfa, 0xab, 0x44, 0x04, 0xcd,
    0x00, 0x04, 0x00, 0x03, 0xff, 0x3b, 0x08, 0x00, 0x05, 0x4c, 0x00, 0x15,
    0x00, 0x1d, 0x00, 0x25, 0x01, 0x1b, 0x00, 0x00, 0x01, 0x36, 0x37, 0x36,
    0x27, 0x26, 0x07, 0x06, 0x06, 0x23, 0x22, 0x27, 0x26, 0x27, 0x26, 0x07,
    0x06, 0x17, 0x16, 0x17, 0x16, 0x32, 0x37, 0x32, 0x35, 0x34, 0x23, 0x22,
    0x15, 0x14, 0x27, 0x32, 0x35, 0x34, 0x23, 0x22, 0x15, 0x14, 0x03, 0x32,
    0x17, 0x30, 0x17, 0x31, 0x36, 0x37, 0x36, 0x37, 0x36, 0x37, 0x36, 0x33,
    0x32, 0x33, 0x16, 0x33, 0x32, 0x17, 0x16, 0x07, 0x06, 0x23, 0x22, 0x27,
    0x26, 0x27, 0x26, 0x23, 0x22, 0x07, 0x07, 0x06, 0x07, 0x06, 0x07, 0x06,
    0x1f, 0x02, 0x37, 0x36, 0x37, 0x36, 0x33, 0x32, 0x17, 0x17, 0x16, 0x33,
    0x16, 0x17, 0x16, 0x07, 0x06, 0x23, 0x22, 0x27, 0x27, 0x26, 0x23, 0x22,
    0x07, 0x06, 0x07, 0x06, 0x17, 0x16, 0x17, 0x16, 0x33, 0x32, 0x33, 0x32,
    0x37, 0x36, 0x37, 0x36, 0x17, 0x16, 0x1f, 0x02, 0x16, 0x17, 0x16, 0x15,
    0x14, 0x23, 0x22, 0x27, 0x27, 0x26, 0x27, 0x27, 0x26, 0x27, 0x26, 0x07,
    0x06, 0x07, 0x06, 0x17, 0x16, 0x17, 0x16, 0x15, 0x14, 0x07, 0x06, 0x07,
    0x06, 0x23, 0x22, 0x27, 0x26, 0x07, 0x06, 0x07, 0x06, 0x15, 0x14, 0x17,
    0x16, 0x17, 0x16, 0x15, 0x14, 0x07, 0x06, 0x23, 0x22, 0x27, 0x26, 0x27,
    0x26, 0x35, 0x34, 0x37, 0x36, 0x37, 0x36, 0x37, 0x34, 0x27, 0x26, 0x07,
    0x06, 0x07, 0x06, 0x0f, 0x02, 0x06, 0x23, 0x22, 0x27, 0x26, 0x35, 0x34,
    0x37, 0x37, 0x36, 0x37, 0x36, 0x37, 0x36, 0x37, 0x36, 0x27, 0x26, 0x27,
    0x26, 0x07, 0x06, 0x07, 0x06, 0x07, 0x06, 0x07, 0x07, 0x06, 0x23, 0x22,
    0x27, 0x26, 0x35, 0x34, 0x37, 0x36, 0x37, 0x37, 0x36, 0x37, 0x37, 0x36,
    0x37, 0x36, 0x37, 0x36, 0x35, 0x34, 0x27, 0x26, 0x27, 0x26, 0x27, 0x26,
    0x23, 0x22, 0x07, 0x06, 0x07, 0x06, 0x07, 0x06, 0x27, 0x26, 0x27, 0x26,
    0x27, 0x26, 0x35, 0x34, 0x37, 0x36, 0x37, 0x36, 0x37, 0x36, 0x33, 0x32,
    0x17, 0x16, 0x33, 0x32, 0x37, 0x36, 0x35, 0x34, 0x37, 0x36, 0x37, 0x36,
    0x33, 0x04, 0xf5, 0x23, 0x13, 0x11, 0x14, 0x16, 0x1d, 0x1b, 0x4c, 0x1f,
    0x0e, 0x2d, 0x23, 0x14, 0x2c, 0x13, 0x18, 0x25, 0x2c, 0x10, 0x3c, 0x71,
    0x1d, 0x5c, 0x5c, 0x3f, 0xae, 0x5c, 0x5c, 0x3f, 0x6a, 0x27, 0x31, 0x5b,
    0x09, 0x27, 0x36, 0x03, 0x0a, 0x26, 0x35, 0x2e, 0x09, 0x08, 0xc6, 0x13,
    0x81, 0x17, 0x20, 0x18, 0x21, 0x1e, 0x04, 0x04, 0x15, 0x5c, 0x22, 0x26,
    0x48, 0x56, 0x3b, 0x10, 0x21, 0x01, 0x0c, 0x06, 0x06, 0x0f, 0x31, 0x44,
    0x3c, 0x52, 0x4a, 0x1d, 0x11, 0x3f, 0xb4, 0x71, 0x01, 0x26, 0x06, 0x0d,
    0x15, 0x1a, 0x2a, 0x13, 0x53, 0xaa, 0x42, 0x1d, 0x0a, 0x33, 0x20, 0x21,
    0x2b, 0x01, 0x02, 0x3e, 0x21, 0x09, 0x02, 0x02, 0x0f, 0x2d, 0x4b, 0x0a,
    0x22, 0x15, 0x20, 0x1f, 0x72, 0x8b, 0x2d, 0x2f, 0x1d, 0x1f, 0x0e, 0x25,
    0x3f, 0x4d, 0x1b, 0x63, 0x2a, 0x2c, 0x14, 0x22, 0x18, 0x1c, 0x0f, 0x08,
    0x2a, 0x08, 0x08, 0x0d, 0x3b, 0x4c, 0x52, 0x74, 0x27, 0x71, 0x2e, 0x01,
    0x0c, 0x10, 0x15, 0x0d, 0x06, 0x0d, 0x05, 0x01, 0x06, 0x2c, 0x28, 0x14,
    0x1b, 0x05, 0x04, 0x10, 0x06, 0x12, 0x08, 0x0a, 0x16, 0x27, 0x03, 0x0d,
    0x30, 0x4c, 0x4c, 0x4b, 0x1f, 0x0b, 0x22, 0x26, 0x0d, 0x15, 0x0d, 0x2d,
    0x68, 0x34, 0x14, 0x3c, 0x25, 0x12, 0x04, 0x10, 0x18, 0x0b, 0x09, 0x30,
    0x2b, 0x44, 0x66, 0x14, 0x47, 0x47, 0x59, 0x73, 0x25, 0x05, 0x03, 0x1f,
    0x01, 0x08, 0x3f, 0x48, 0x4b, 0x4b, 0x76, 0x2f, 0x49, 0x2d, 0x22, 0x24,
    0x0c, 0x15, 0x08, 0x0e, 0x33, 0x03, 0x44, 0x4c, 0x10, 0x46, 0x13, 0x1f,
    0x27, 0x1b, 0x1d, 0x13, 0x02, 0x24, 0x08, 0x02, 0x42, 0x0e, 0x4d, 0x3c,
    0x19, 0x1b, 0x40, 0x2b, 0x2b, 0x1e, 0x16, 0x11, 0x04, 0x1f, 0x11, 0x04,
    0x18, 0x11, 0x35, 0x01, 0xa3, 0x13, 0x24, 0x1f, 0x0b, 0x0c, 0x19, 0x19,
    0x18, 0x13, 0x0f, 0x0c, 0x1a, 0x18, 0x1f, 0x19, 0x1e, 0x07, 0x1a, 0xc3,
    0x54, 0x51, 0x54, 0x51, 0x04, 0x53, 0x51, 0x54, 0x50, 0x02, 0x48, 0x1a,
    0x31, 0x18, 0x55, 0x74, 0x04, 0x0e, 0x09, 0x0d, 0x06, 0x10, 0x16, 0x1b,
    0x24, 0x01, 0x04, 0x0b, 0x04, 0x10, 0x3f, 0x0a, 0x41, 0x02, 0x41, 0x20,
    0x06, 0x12, 0x16, 0x21, 0x17, 0x2a, 0x1e, 0x15, 0x40, 0x27, 0x11, 0x0e,
    0x1e, 0x11, 0x15, 0x1f, 0x43, 0x13, 0x1a, 0x10, 0x15, 0x1b, 0x04, 0x09,
    0x4d, 0x2a, 0x0f, 0x19, 0x0a, 0x0a, 0x03, 0x05, 0x15, 0x3c, 0x64, 0x21,
    0x4b, 0x2e, 0x21, 0x28, 0x13, 0x47, 0x44, 0x19, 0x3f, 0x11, 0x18, 0x0b,
    0x0a, 0x07, 0x18, 0x0d, 0x07, 0x24, 0x2c, 0x2b, 0x21, 0x32, 0x10, 0x48,
    0x2a, 0x2d, 0x1e, 0x1a, 0x01, 0x0c, 0x43, 0x59, 0x28, 0x4e, 0x1c, 0x0d,
    0x5d, 0x24, 0x14, 0x0a, 0x05, 0x1f, 0x24, 0x32, 0x46, 0x3e, 0x5f, 0x3e,
    0x44, 0x1a, 0x30, 0x15, 0x0d, 0x07, 0x18, 0x2b, 0x03, 0x0d, 0x1a, 0x28,
    0x28, 0x57, 0xb2, 0x29, 0x27, 0x40, 0x2c, 0x23, 0x16, 0x63, 0x58, 0x1a,
    0x0a, 0x18, 0x11, 0x23, 0x08, 0x1b, 0x29, 0x05, 0x04, 0x0b, 0x15, 0x0d,
    0x14, 0x0b, 0x2a, 0x29, 0x5a, 0x62, 0x01, 0x19, 0x1e, 0x05, 0x05, 0x26,
    0x42, 0x42, 0x2a, 0x2a, 0x3f, 0x0d, 0x0f, 0x09, 0x05, 0x07, 0x01, 0x0b,
    0x25, 0x3e, 0x0d, 0x17, 0x11, 0x01, 0x03, 0x0d, 0x13, 0x20, 0x19, 0x11,
    0x03, 0x02, 0x01, 0x04, 0x11, 0x04, 0x05, 0x1b, 0x3d, 0x10, 0x29, 0x20,
    0x04, 0x04, 0x0a, 0x07, 0x04, 0x1f, 0x15, 0x20, 0x3e, 0x0f, 0x2a, 0x1e,
    0x00, 0x00, 0x00, 0x1b, 0x01, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x01, 0x00, 0x0c, 0x00, 0x1b, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x02, 0x00, 0x07, 0x00, 0x27, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x03, 0x00, 0x0c, 0x00, 0x1b, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x04, 0x00, 0x0c, 0x00, 0x1b, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x05, 0x00, 0x02, 0x00, 0x2e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x06, 0x00, 0x0c, 0x00, 0x1b, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0d, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0e, 0x00, 0x1a, 0x00, 0x30, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x00, 0x00, 0x36, 0x00, 0x4a, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x01, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x02, 0x00, 0x0e, 0x00, 0x98, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x03, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x04, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x05, 0x00, 0x04, 0x00, 0xa6, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x06, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x0d, 0x00, 0x36, 0x00, 0x4a, 0x00, 0x03, 0x00, 0x00, 0x04, 0x09,
    0x00, 0x0e, 0x00, 0x34, 0x00, 0xaa, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x00, 0x00, 0x36, 0x00, 0x4a, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x01, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x02, 0x00, 0x0e, 0x00, 0x98, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x03, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x04, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x05, 0x00, 0x04, 0x00, 0xa6, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x06, 0x00, 0x18, 0x00, 0x80, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x0d, 0x00, 0x36, 0x00, 0x4a, 0x00, 0x03, 0x00, 0x01, 0x04, 0x09,
    0x00, 0x0e, 0x00, 0x34, 0x00, 0xaa, 0x43, 0x6f, 0x70, 0x79, 0x72, 0x69,
    0x67, 0x68, 0x74, 0x20, 0x28, 0x63, 0x29, 0x20, 0x32, 0x30, 0x31, 0x35,
    0x2c, 0x20, 0x47, 0x6f, 0x6f, 0x67, 0x6c, 0x65, 0x2e, 0x53, 0x70, 0x69,
    0x64, 0x65, 0x72, 0x53, 0x79, 0x6d, 0x62, 0x6f, 0x6c, 0x52, 0x65, 0x67,
    0x75, 0x6c, 0x61, 0x72, 0x56, 0x31, 0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f,
    0x2f, 0x73, 0x63, 0x72, 0x69, 0x70, 0x74, 0x73, 0x2e, 0x73, 0x69, 0x6c,
    0x2e, 0x6f, 0x72, 0x67, 0x2f, 0x4f, 0x46, 0x4c, 0x00, 0x43, 0x00, 0x6f,
    0x00, 0x70, 0x00, 0x79, 0x00, 0x72, 0x00, 0x69, 0x00, 0x67, 0x00, 0x68,
    0x00, 0x74, 0x00, 0x20, 0x00, 0x28, 0x00, 0x63, 0x00, 0x29, 0x00, 0x20,
    0x00, 0x32, 0x00, 0x30, 0x00, 0x31, 0x00, 0x35, 0x00, 0x2c, 0x00, 0x20,
    0x00, 0x47, 0x00, 0x6f, 0x00, 0x6f, 0x00, 0x67, 0x00, 0x6c, 0x00, 0x65,
    0x00, 0x2e, 0x00, 0x53, 0x00, 0x70, 0x00, 0x69, 0x00, 0x64, 0x00, 0x65,
    0x00, 0x72, 0x00, 0x53, 0x00, 0x79, 0x00, 0x6d, 0x00, 0x62, 0x00, 0x6f,
    0x00, 0x6c, 0x00, 0x52, 0x00, 0x65, 0x00, 0x67, 0x00, 0x75, 0x00, 0x6c,
    0x00, 0x61, 0x00, 0x72, 0x00, 0x56, 0x00, 0x31, 0x00, 0x68, 0x00, 0x74,
    0x00, 0x74, 0x00, 0x70, 0x00, 0x3a, 0x00, 0x2f, 0x00, 0x2f, 0x00, 0x73,
    0x00, 0x63, 0x00, 0x72, 0x00, 0x69, 0x00, 0x70, 0x00, 0x74, 0x00, 0x73,
    0x00, 0x2e, 0x00, 0x73, 0x00, 0x69, 0x00, 0x6c, 0x00, 0x2e, 0x00, 0x6f,
    0x00, 0x72, 0x00, 0x67, 0x00, 0x2f, 0x00, 0x4f, 0x00, 0x46, 0x00, 0x4c,
    0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x66,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0xff, 0xff, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0c, 0x00, 0x14, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x02, 0x00, 0x00
};

/**
 * There does not appear to be a publicly accessible API for determining if lcd
 * font smoothing will be applied if we request it. The main issue is that if
 * smoothing is applied a gamma of 2.0 will be used, if not a gamma of 1.0.
 */
SkCTFontSmoothBehavior SkCTFontGetSmoothBehavior() {
    static SkCTFontSmoothBehavior gSmoothBehavior = []{
        uint32_t noSmoothBitmap[16][16] = {};
        uint32_t smoothBitmap[16][16] = {};

        SkUniqueCFRef<CGColorSpaceRef> colorspace(CGColorSpaceCreateDeviceRGB());
        SkUniqueCFRef<CGContextRef> noSmoothContext(
                CGBitmapContextCreate(&noSmoothBitmap, 16, 16, 8, 16*4,
                                      colorspace.get(), kBitmapInfoRGB));
        SkUniqueCFRef<CGContextRef> smoothContext(
                CGBitmapContextCreate(&smoothBitmap, 16, 16, 8, 16*4,
                                      colorspace.get(), kBitmapInfoRGB));

        SkUniqueCFRef<CFDataRef> data(CFDataCreateWithBytesNoCopy(
                kCFAllocatorDefault, kSpiderSymbol_ttf, std::size(kSpiderSymbol_ttf),
                kCFAllocatorNull));
        SkUniqueCFRef<CTFontDescriptorRef> desc(
                CTFontManagerCreateFontDescriptorFromData(data.get()));
        SkUniqueCFRef<CTFontRef> ctFont(CTFontCreateWithFontDescriptor(desc.get(), 16, nullptr));
        SkASSERT(ctFont);

        CGContextSetShouldSmoothFonts(noSmoothContext.get(), false);
        CGContextSetShouldAntialias(noSmoothContext.get(), true);
        CGContextSetTextDrawingMode(noSmoothContext.get(), kCGTextFill);
        CGContextSetGrayFillColor(noSmoothContext.get(), 1, 1);

        CGContextSetShouldSmoothFonts(smoothContext.get(), true);
        CGContextSetShouldAntialias(smoothContext.get(), true);
        CGContextSetTextDrawingMode(smoothContext.get(), kCGTextFill);
        CGContextSetGrayFillColor(smoothContext.get(), 1, 1);

        CGPoint point = CGPointMake(0, 3);
        CGGlyph spiderGlyph = 3;
        CTFontDrawGlyphs(ctFont.get(), &spiderGlyph, &point, 1, noSmoothContext.get());
        CTFontDrawGlyphs(ctFont.get(), &spiderGlyph, &point, 1, smoothContext.get());

        // For debugging.
        //SkUniqueCFRef<CGImageRef> image(CGBitmapContextCreateImage(noSmoothContext()));
        //SkUniqueCFRef<CGImageRef> image(CGBitmapContextCreateImage(smoothContext()));

        SkCTFontSmoothBehavior smoothBehavior = SkCTFontSmoothBehavior::none;
        for (int x = 0; x < 16; ++x) {
            for (int y = 0; y < 16; ++y) {
                uint32_t smoothPixel = smoothBitmap[x][y];
                uint32_t r = (smoothPixel >> 16) & 0xFF;
                uint32_t g = (smoothPixel >>  8) & 0xFF;
                uint32_t b = (smoothPixel >>  0) & 0xFF;
                if (r != g || r != b) {
                    return SkCTFontSmoothBehavior::subpixel;
                }
                if (noSmoothBitmap[x][y] != smoothPixel) {
                    smoothBehavior = SkCTFontSmoothBehavior::some;
                }
            }
        }
        return smoothBehavior;
    }();
    return gSmoothBehavior;
}

SkCTFontWeightMapping& SkCTFontGetNSFontWeightMapping() {
    // In the event something goes wrong finding the real values, use this mapping.
    static constexpr CGFloat defaultNSFontWeights[] =
        { -1.00, -0.80, -0.60, -0.40, 0.00, 0.23, 0.30, 0.40, 0.56, 0.62, 1.00 };

    // Declarations in <AppKit/AppKit.h> on macOS, <UIKit/UIKit.h> on iOS
#ifdef SK_BUILD_FOR_MAC
#  define SK_KIT_FONT_WEIGHT_PREFIX "NS"
#endif
#ifdef SK_BUILD_FOR_IOS
#  define SK_KIT_FONT_WEIGHT_PREFIX "UI"
#endif
    static constexpr const char* nsFontWeightNames[] = {
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightUltraLight",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightThin",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightLight",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightRegular",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightMedium",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightSemibold",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightBold",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightHeavy",
        SK_KIT_FONT_WEIGHT_PREFIX "FontWeightBlack",
    };
    static_assert(std::size(nsFontWeightNames) == 9, "");

    static CGFloat nsFontWeights[11];
    static const CGFloat (*selectedNSFontWeights)[11] = &defaultNSFontWeights;
    static SkOnce once;
    once([&] {
        size_t i = 0;
        nsFontWeights[i++] = -1.00;
        for (const char* nsFontWeightName : nsFontWeightNames) {
            void* nsFontWeightValuePtr = dlsym(RTLD_DEFAULT, nsFontWeightName);
            if (nsFontWeightValuePtr) {
                nsFontWeights[i++] = *(static_cast<CGFloat*>(nsFontWeightValuePtr));
            } else {
                return;
            }
        }
        nsFontWeights[i++] = 1.00;
        selectedNSFontWeights = &nsFontWeights;
    });
    return *selectedNSFontWeights;
}

SkCTFontWeightMapping& SkCTFontGetDataFontWeightMapping() {
    // In the event something goes wrong finding the real values, use this mapping.
    // These were the values from macOS 10.13 to 10.15.
    static constexpr CGFloat defaultDataFontWeights[] =
        { -1.00, -0.70, -0.50, -0.23, 0.00, 0.20, 0.30, 0.40, 0.60, 0.80, 1.00 };

    static const CGFloat (*selectedDataFontWeights)[11] = &defaultDataFontWeights;
    static CGFloat dataFontWeights[11];
    static SkOnce once;
    once([&] {
        constexpr size_t dataSize = std::size(kSpiderSymbol_ttf);
        sk_sp<SkData> data = SkData::MakeWithCopy(kSpiderSymbol_ttf, dataSize);
        const SkSFNTHeader* sfntHeader = reinterpret_cast<const SkSFNTHeader*>(data->data());
        const SkSFNTHeader::TableDirectoryEntry* tableEntry =
            SkTAfter<const SkSFNTHeader::TableDirectoryEntry>(sfntHeader);
        const SkSFNTHeader::TableDirectoryEntry* os2TableEntry = nullptr;
        int numTables = SkEndian_SwapBE16(sfntHeader->numTables);
        for (int tableEntryIndex = 0; tableEntryIndex < numTables; ++tableEntryIndex) {
            if (SkOTTableOS2::TAG == tableEntry[tableEntryIndex].tag) {
                os2TableEntry = tableEntry + tableEntryIndex;
                break;
            }
        }
        if (!os2TableEntry) {
            return;
        }
        size_t os2TableOffset = SkEndian_SwapBE32(os2TableEntry->offset);
        SkOTTableOS2_V0* os2Table = SkTAddOffset<SkOTTableOS2_V0>(data->writable_data(),
                                                                  os2TableOffset);

        CGFloat previousWeight = -CGFLOAT_MAX;
        for (int i = 0; i < 11; ++i) {
            os2Table->usWeightClass.value = SkEndian_SwapBE16(i * 100);

            // On macOS 10.14 and earlier it appears that the CFDataGetBytePtr is used somehow in
            // font caching. Creating a slightly modified font with data at the same address seems
            // to in some ways act like a font previously created at that address. As a result,
            // always make a copy of the data.
            SkUniqueCFRef<CFDataRef> cfData(
                    CFDataCreate(kCFAllocatorDefault, (const UInt8 *)data->data(), data->size()));
            if (!cfData) {
                return;
            }
            SkUniqueCFRef<CTFontDescriptorRef> desc(
                    CTFontManagerCreateFontDescriptorFromData(cfData.get()));
            if (!desc) {
                return;
            }

            // On macOS 10.14 and earlier, the CTFontDescriptorRef returned from
            // CTFontManagerCreateFontDescriptorFromData is incomplete and does not have the
            // correct traits. It is necessary to create the CTFont and then get the descriptor
            // off of it.
            SkUniqueCFRef<CTFontRef> ctFont(CTFontCreateWithFontDescriptor(desc.get(), 9, nullptr));
            if (!ctFont) {
                return;
            }
            SkUniqueCFRef<CTFontDescriptorRef> desc2(CTFontCopyFontDescriptor(ctFont.get()));
            if (!desc2) {
                return;
            }

            SkUniqueCFRef<CFTypeRef> traitsRef(
                    CTFontDescriptorCopyAttribute(desc2.get(), kCTFontTraitsAttribute));
            if (!traitsRef || CFGetTypeID(traitsRef.get()) != CFDictionaryGetTypeID()) {
                return;
            }
            CFDictionaryRef fontTraitsDict = static_cast<CFDictionaryRef>(traitsRef.get());

            CFTypeRef weightRef;
            if (!CFDictionaryGetValueIfPresent(fontTraitsDict, kCTFontWeightTrait, &weightRef) ||
                !weightRef)
            {
                return;
            }

            // It is possible there is a kCTFontWeightTrait entry, but it is not a CFNumberRef.
            // This is usually due to a bug with the handling of 0, so set the default to 0.
            // See https://crbug.com/1372420
            CGFloat weight = 0;
            if (CFGetTypeID(weightRef) == CFNumberGetTypeID()) {
                CFNumberRef weightNumber = static_cast<CFNumberRef>(weightRef);
                if (!CFNumberIsFloatType(weightNumber) ||
                    !CFNumberGetValue(weightNumber, kCFNumberCGFloatType, &weight))
                {
                    // CFNumberGetValue may modify `weight` even when returning `false`.
                    weight = 0;
                }
            }

            // It is expected that the weights will be strictly monotonically increasing.
            if (weight <= previousWeight) {
                return;
            }
            previousWeight = weight;
            dataFontWeights[i] = weight;
        }
        selectedDataFontWeights = &dataFontWeights;
    });
    return *selectedDataFontWeights;
}

#endif
