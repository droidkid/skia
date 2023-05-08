# SkiLang

NOTE: This is not exact SkiLang.

## SkiLang Operators

### Concat

```
---- SkRecord ----

drawRectA
drawRectB
drawRectC

---- SkiLang -----

(concat
    (concat
        drawRectA
        drawRectB
    )
    drawRectC
)
```

### ClipRect, ConcatM44, MatrixOp

```
---- SkRecord ----

drawRectA
ClipRect
drawRectB
Scale 2.0
drawRectC

---- SkiLang -----

(concat
    (concat
        drawRectA
        (ClipRect drawRectB)
    )
    (ClipRect (Scale  drawRectC) )
)

OR (Rewrite rules!)

(concat
    drawRectA
    (ClipRect
        (concat
            drawRectB
            (Scale drawRectC)
        )
    )
)
```

### Save, Restore

```
---- SkRecord ----

drawRectA
ClipRect
drawRectB
Save
    Scale 2.0
    drawRectC
Restore
drawRectD

---- SkiLang -----

(concat
    (concat
        (concat
            drawRectA
            (ClipRect drawRectB)
        )
        (ClipRect (Scale  drawRectC ) )
    )
    (ClipRect drawRectD)
)

OR (Rewrite rules!)

(concat
    drawRectA
    (ClipRect
        (concat
            (concat
                drawRectB
                (Scale drawRectC)
            )
            drawRectD
        )
    )
)
```


### SaveLayer, Restore (and VirtualOps)

```
---- SkRecord ----

drawRectA
SaveLayer(merge:srcOver, bounds)
    drawRectB
    drawRectC
Restore
drawRectC

---- SkiLang -----

(concat
    (merge
        drawRectA
        (concat drawRectB drawRectC)
        [srcOver, bounds]
        blankState
    )
    drawRectC
)

```
```
---- SkRecord ----

drawRectA
ClipRect
Scale 2.0
drawRectB
SaveLayer(merge:srcOver, bounds, gauss_blur)
    Translate (x, y)
    drawRectC
    drawRectD
Restore
drawRectE

---- SkiLang -----

(concat
    (merge
        (concat drawRectA (Clip (Scale (drawRectB))))
        (translate
            (concat drawRectB drawRectC)
        )
        [srcOver, bounds, gauss_blur]
        (ClipRect (Scale (~)))
    )
    (ClipRect (Scale drawRectC) )
)

---- IF WE HAD VIRTUAL OPS FOR srcOver, bounds, gauss_blur ---

(srcOver
    (srcOver
        drawRectA

        (ClipRect
        (Scale
        (ClipRectToLayerBounds
        (GaussBlur
            (Translate
                (concat drawRectB drawRectC)
            )
        ))))
    )
    (ClipRect(Scale( drawRectC) ))
)


```

Why is ClipToBounds separate from ClipRect? They could be merged. 
Mainly because we don't have a rewrite rule that says
```
(merge
    dst
    (clipRect src clipRectBounds)
    [mergeParams.., noBounds]
) ->
(merge
    dst,
    src
    [mergeParams.., clipRectBounds]
)
```

### Rewrite rules

#### Blank Rules

#### Associativity

```
(srcOver A (srcOver B C)) <=> (srcOver (srcOver A B) C)
```

#### Commutativity
```
(clipRect (alpha surface)) => (alpha (clipRect surface))
```

#### SaveLayer To VirtualOps Rules

#### Common Rules
```
(concat
    (ClipRect surface1 bounds)
    (ClipRect surface2 bounds)
) =>
(ClipRect
    (concat surface1 surface2)
    bounds
)
```

##### Merge Rules

```
(ClipRect (ClipRect surface b1) b2) =>
(ClipRect surface b1 intersect b2)

(Alpha (Alpha surface a2) a1) =>
(Alpha surface a1*a2/255 )

```

