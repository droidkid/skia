# SkiPass (Optimizing Skia with Egraphs)

SkiPass attempts to optimize [Skia](https://skia.org) SkPictures using Egraphs (via [egg](https://egraphs-good.github.io/)).

## QuickStart

```bash
make local-nightly # Generates report in ./skia_opt_research/out
```


## SkiPass Details 

### High Level Overview

![image](./skia_opt_research/docs/overview.png)

In Skia, a [SkPicture](https://api.skia.org/classSkPicture.html) or SKP, is a recording of draw commands (internally called [SkRecords](https://source.chromium.org/chromium/chromium/src/+/main:third_party/skia/src/core/SkRecords.h?q=SkRecords&ss=chromium)).

SkiPass converts a sequential list of draw commands in a SKP to a functional representation, which we call SkiLang of the final image rendered by the SKP. This functional representation is optimized using [egg](https://github.com/egraphs-good/egg), a e-graph library using a collection of simple equivalence functional rules. 

### SkiPassOptimize Flow

![image](./skia_opt_research/docs/SkiPassOptimize.png)


TODO: Add in locations of the above participant files 

TODO: Show some example SkiLang translations and optimizations 

## Benchmark Details

TODO: How is memory measured? (mention we don't count memory reuse currently)


-------------
(Below follows the original Skia README)

--------

Skia is a complete 2D graphic library for drawing Text, Geometries, and Images.

See full details, and build instructions, at https://skia.org.
