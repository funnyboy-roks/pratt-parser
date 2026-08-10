// vim: syntax=rust

for y in 0..5 {
    for x in 0..5 {
        if x == 2 {
            continue;
        }
        print(x, y);
    }
}
