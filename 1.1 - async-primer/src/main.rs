use futures::executor::block_on;

// To create an asynchronous function, you can use the async fn syntax
async fn hello_world() {
    println!("hello, world!")
}

fn basic_example() {
    // The value returned by async fn is a Future.
    // For anything to happen, the Future needs to be run on an executor.
    let future = hello_world(); // Nothing is print
    block_on(future);   // `future` is run and "hello, world!" is printed
}

// Inside an async fn, you can use .await to wait for the completion of another type that implements the Future trait,
// such as the output of another async fn. Unlike block_on, .await doesn't block the current thread,
// but instead asynchronously waits for the future to complete,
// allowing other tasks to run if the future is currently unable to make progress.

// In this example, learning the song must happen before singing the song,
// but both learning and singing can happen at the same time as dancing.
#[derive(Debug)]
struct Song {
    title: String,
    singer: String
}

impl Song {
    fn new() -> Song {
        Song {
            title: String::from(""),
            singer: String::from(""),
        }
    }
}

async fn learn_song() -> Song {
    Song::new()
}
async fn sing_song(song: Song) {
    println!("Learning song: {:#?}", song)
}
async fn dance() {
    println!("Dance!")
}

async fn learn_and_sing() {
    // Wait until the song has been learned before singing it.
    // We use `.await` here rather than `block_on` to prevent blocking the
    // thread, which makes it possible to `dance` at the same time.
    let song = learn_song().await;
    sing_song(song).await;
}

async fn another_example() {
    let f1 = learn_and_sing();
    let f2 = dance();

    // `join!` is like `.await` but can wait for multiple futures concurrently.
    // If we're temporarily blocked in the `learn_and_sing` future, the `dance`
    // future will take over the current thread. If `dance` becomes blocked,
    // `learn_and_sing` can take back over. If both futures are blocked, then
    // this function is blocked and will yield to the executor.
    futures::join!(f1, f2);
}

fn main() {
    basic_example();
    block_on(another_example());
}
