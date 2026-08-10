# Pratt Parser

A toy interpreter created so I can play around with Pratt Parsing
(implementation based on <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>)

Fibonacci Numbers in this language (using recursion):

```rust
let f = (n) => if n <= 2 { 1 } else { f(n - 1) + f(n - 2) };

f(10) // 55
```

Or using iteration

```rust
let a = 1;
let b = 1;
for i in 0..10 {
    print(a);
    b = a + b;
    a = b - a;
}
```
