# SkiPass

SkiPass is a Skia SKP optimizer.  

TODO(chesetti): Add more details.

Exmample run:

```bash
$ cargo run ./res/skp3.json out.skp
```

### SkiLang

TODO(chesetti): This is out of date.

```rust
// SkiLang 
define_language! {
    pub enum SkiLang {
        Num(i32),
        "point" = Point([Id; 2]), // X, Y
        "dimensions" = Dim([Id; 2]), // W, H
        "color" = Color([Id; 4]), // argb, 0-255
        "paint" = Paint([Id; 1]), // color
        "blank" = Blank, // dimensions
        "srcOver" = SrcOver([Id; 2]), // dst, src
        "drawRect" = DrawRect([Id; 3]), // top_point, bot_point, paint
    }
}

```

Example JSON
```json
{
   "version": 1,
   "commands": [
      {
         "command": "DrawRect",
         "visible": true,
         "coords": [ 10, 70, 60, 120 ],
         "paint": {
            "color": [ 255, 255, 0, 0 ]
         },
         "shortDesc": " [10 70 60 120]"
      },
      {
         "command": "DrawRect",
         "visible": true,
         "coords": [ 150, 70, 200, 120 ],
         "paint": {
            "color": [ 255, 255, 0, 0 ]
         },
         "shortDesc": " [150 70 200 120]"
      },
      {
         "command": "SaveLayer",
         "visible": true,
         "paint": {
            "color": [ 77, 0, 0, 0 ]
         }
      },
      {
         "command": "DrawRect",
         "visible": true,
         "coords": [ 30, 70, 80, 120 ],
         "paint": {
            "color": [ 255, 0, 0, 255 ]
         },
         "shortDesc": " [30 70 80 120]"
      },
      {
         "command": "DrawRect",
         "visible": true,
         "coords": [ 170, 70, 220, 120 ],
         "paint": {
            "color": [ 77, 0, 0, 255 ]
         },
         "shortDesc": " [170 70 220 120]"
      },
      {
         "command": "Restore",
         "visible": true
      }
   ]
}
```

Output:
```
$ cargo run ./res/skp3.json
DrawRect { coords: [10, 70, 60, 120], paint: SkPaint { color: [255, 255, 0, 0] }, visible: true }
DrawRect { coords: [150, 70, 200, 120], paint: SkPaint { color: [255, 255, 0, 0] }, visible: true }
SaveLayer { paint: SkPaint { color: [77, 0, 0, 0] }, visible: true }
DrawRect { coords: [30, 70, 80, 120], paint: SkPaint { color: [255, 0, 0, 255] }, visible: true }
DrawRect { coords: [170, 70, 220, 120], paint: SkPaint { color: [77, 0, 0, 255] }, visible: true }
Restore { visible: true }
(srcOver
  (srcOver
    (srcOver
      blank
      (drawRect
        (point 10 70)
        (point 60 120)
        (paint (color 255 255 0 0))))
    (drawRect
      (point 150 70)
      (point 200 120)
      (paint (color 255 255 0 0))))
  (srcOver
    (srcOver
      blank
      (drawRect
        (point 30 70)
        (point 80 120)
        (paint (color 255 0 0 255))))
    (drawRect
      (point 170 70)
      (point 220 120)
      (paint (color 77 0 0 255)))))
```



## How to generate JSON

Use the skp_parser program in the tools/skia directory. Some sample JSONs are in the res/ directory.

#### Why convert to JSON, Why not just read a .skp? 

The Skia source code has tools to dump JSON, but doesn't seem to have a way to convert JSON to SKP.
The Skia rust bindings seem to expose only the public API (or I just haven't figured out how yet), so parsing individual draw commands from a Skp in Rust looks troublesome.

I also faced a issue where I could not parse an .skp I downloaded from fiddle.skia.org (version issues maybe?). I ended up generating a .skp locally and using that. JSON is simpler to edit and iterate over the stuff we want to try.