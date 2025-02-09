fn counter(x: usize) {
    let mut i = 0;
    (0..x).for_each(|_| {
        print!("Counter [{}]: {} :: ", coroutines::id(), i);
        i += 1;
        println!("Yielding");
        coroutines::yield_coroutine();
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    coroutines::go(|| counter(10));
    coroutines::go(|| counter(15));
    coroutines::go(|| counter(20));
    coroutines::go(|| counter(25));

    while coroutines::alive() > 1 {
        coroutines::yield_coroutine();
    }
    println!("All coroutines finished");

    Ok(())
}
