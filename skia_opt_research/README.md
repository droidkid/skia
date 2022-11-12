## Running the benchmark

From the skia root directory, assuming the build directory is `out/Debug`
Make sure you've checkout `SkiaOpt`

```bash
$ make nightly
```

This will generate a directory with HTML report in `skia_opt_research/out/<YYYY-MM-DD-mm-hh-ss>`

## Adding a new Skp

Add a new draw test_case to `gen_skp.cpp`.

Add a raster call to the main function in `gen_skp.cpp`.

From the skia root directory, assuming the build directory is `out/Debug`

```bash
$ ninja -C ./out/Debug skp_opt_gen_skps
$ ./out/Debug skp_opt_gen_skps
```

## Generating proto source files

```
$ make gen-proto
```