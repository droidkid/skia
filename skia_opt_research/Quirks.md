Notes taken while trying to make the C++ and Rust bindings output the same SKP.

## C++ Skia Canvas seems to optimize this case 

```c++
canvas.saveLayer(nullptr, nullptr);
canvas.drawRect(...);
canvas.restore();

// The above is optimized to the below in the SKP. 

canvas.drawRect(...);
```

## C++ does not seem to optimize empty save layer

## Rust Skia Bindings seems to optimize this case 

```rust
canvas.save_layer(&SaveLayerRect::default());
canvas.restore();

// The above is nuked in the SKP.
```

### But this is not optimized...

```rust
canvas.save_layer_alpha(None, (0 as usize).into);
canvas.restore();

// The written out SKP has a save_layer, restore pair.
```

### Questions

1. Where do these optimizations happen?