// vim: syntax=rust

let a = 1;
let b = 1;
for i in 0..10 {
    print(a);
    b = a + b;
    a = b - a;
}
