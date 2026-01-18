#[test]
fn test_for_loop() {
    let n = 1000;
    let mut res = 0;

    for i in 1..=n {
        res += i;
    }

    println!("res = {}", res);
}

#[test]
fn test_while_loop() {
    let n = 1000;
    let mut res = 0;
    let mut i = 0;

    while i < n {
        res += i;
        i += 1;
    }

    println!("res = {}", res);
}

#[test]
fn test_nested_for_loop() {
    let n = 1000;
    let mut res = vec![];
    for i in 1..=n {
        for j in 1..=n {
            res.push(format!("({}, {})", i, j));
        }
    }

    let joined = res.join("");
    println!("joined = {}", joined);
}
