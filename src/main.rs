use corus::coroutines_custom;

fn counter(x: usize) {
    let mut i = 0;
    (0..x).for_each(|_| {
        let handle = coroutines_custom::handle(coroutines_custom::id());
        print!("Counter_custom [{:?}]: {} :: ", handle.f_ref, i);
        i += 1;
        println!("Yielding");
        coroutines_custom::yield_coroutine();
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    coroutines_custom::spawn(|| counter(10));
    coroutines_custom::spawn(|| counter(15));
    coroutines_custom::spawn(|| counter(20));
    coroutines_custom::spawn(|| counter(25));

    while coroutines_custom::alive() > 1 {
        coroutines_custom::yield_coroutine();
    }
    println!("All coroutines_custom finished");
    Ok(())
}
