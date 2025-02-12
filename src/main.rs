use corus::coroutines;

fn counter(x: usize) {
    let mut i = 0;
    (0..x).for_each(|_| {
        let handle = coroutines::handle(coroutines::id());
        print!("Counter [{:?}]: {} :: ", handle.f_ref, i);
        i += 1;
        println!("Yielding");
        coroutines::yield_coroutine();
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    coroutines::spawn(|| counter(10));
    coroutines::spawn(|| counter(15));
    coroutines::spawn(|| counter(20));
    coroutines::spawn(|| counter(25));

    while coroutines::alive() > 1 {
        coroutines::yield_coroutine();
    }
    println!("All coroutines_custom finished");

    Ok(())
}
