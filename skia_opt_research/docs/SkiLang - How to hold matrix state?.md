
## SkiLang
```
surface -> list of drawCommands
srcOver(dst src) -> return a new surface after drawing src onto dst
```

### Simple Draw - Example 1

```
0 DrawRect
```

```
srcOver(blank DrawCommand(0))
```


### Simple Draw - Example 2

```
0 DrawRect 
1 DrawRect
2 DrawRect
```

```
srcOver(blank
srcOver(DrawCommand(0)
srcOver(DrawCommand(1)
		DrawCommand(2)
)))
```

### Simple Draw - Example 3

```
0 DrawRect
1 SaveLayer
2  DrawRect
3  DrawRect
4 Restore
```

```
srcOver(blank
srcOver(DrawCommand(0)
  srcOver(blank
  srcOver(DrawCommand(2)
		  DrawCommand(3)
  ))
))
```

Notice how the nesting is different in 2 & 3.
