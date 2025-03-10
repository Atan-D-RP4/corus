use corus::generators;
use corus::coroutines;

fn counter(x: usize) {
    let mut i = 0;
    (0..x).for_each(|_| {
        // let handle = coroutines::handle(coroutines::id());
        print!("Counter [{}]: {} :: ", coroutines::id(), i);
        i += 1;
        println!("Yielding");
        coroutines::yield_coroutine();
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    coroutines::spawn(|| counter(5));
    coroutines::spawn(|| counter(10));

    while coroutines::alive() > 1 {
        coroutines::yield_coroutine();
    }
    println!("All coroutines_custom finished");

    generators::example();

    Ok(())
}
