Skia programs hold state in 2 matrices - one for the clip matrix, and another for transformation matrix.


Given the below program, where matrixOp represents a draw command that modifies state matrices (either clip or transform matrices)

```c
0 matrixOp
1 matrixOp
2 DrawRect 
3 Save
4   matrixOp
5   DrawRect
6 Restore
7 Save
8   matrixOp
9   DrawRect
10 Restore
11 DrawRect
```

How does the state carry over to SkiLang?

## Option 1 - attach it to the drawCommand

The above program can also be written as 

```c
0 matrixOp
1 matrixOp
2 DrawRect        (apply matrixOp(0) and matrixOp(1))
3 Save
4   matrixOp
5   DrawRect      (apply matrixOp(0), matrixOp(1) and matrixOp(4))
6 Restore
7 Save
8   matrixOp
9   DrawRect      (apply matrixOp(0), matrixOp(1) and matrixOp(9))
10 Restore
11 DrawRect       (apply matrixOp(0) and matrixOp(1))
```

Knowing which transform calls effect which drawCommand, we can now attach it to a corresponding drawCall

```lisp
srcOver(blank
srcOver(drawRect(2 matrixOp(1 matrixOp(0 i)))
srcOver(drawRect(5 matrixOp(4 matrixOp(1 matrixOp(0 i))))
srcOver(drawRect(9 matrixOp(4 matrixOp(1 matrixOp(0 i))))
srcOver(drawRect(11 matrixOp(1 matrixOp(0 i))))
))))
```


To transform back into Skia DrawCommands
```lisp
surface -> (list of matrix ops, list of draw commands)
srcOver(dst src) ->  draw dst onto src
```

Below is an example of how translating srcOver over to Skia commands could look like in a very simple manner

```lisp
srcOver(
	((matrixOp 2 (matrixOp 1 id) drawCommand(a)) -> dst
	((matrixOp 1 id)             drawCommand(b)) -> src
)
```

```c
save
	(matrixOp 1)
	(matrixOp 2)
	drawCommand(a)
restore
save
	(matrixOp 1)
	drawCommand(a)
restore
```

 In the above example, matrixOp 1 is a common prefix transform, so that can be lifted up when translating back to result in.

```c
(matrixOp 1)
save
	(matrixOp 2)
	drawCommand(a)
restore
drawCommand(b)
```



## Option 2 - attach it as an operator


```c
0 matrixOp
1 matrixOp
2 DrawRect 
3 Save
4   matrixOp
5   DrawRect
6 Restore
7 Save
8   matrixOp
9   DrawRect
10 Restore
11 DrawRect
```

Gets translated to

```
matrixOp(0 
matrixOp(1
srcOver(drawCommand(2)
srcOver(
	matrixOp(4 drawCommand(5))
srcOver(
	matrixOp(8 drawCommand(9))
	drawRect(11)
)))))
```

where 

```
matrixOp(op surface) -> returns new surface with op applied
```

What I haven't figured out about this approach is where to save and restore when translating the SkiLang program back to Skia draw commands.

```c
0 matrixOp
1 matrixOp
2 DrawRect 
3 Save
4   matrixOp
    matrixOp
5   DrawRect
6 Restore
7 Save
8   matrixOp
9   DrawRect
10 Restore
11 DrawRect
```