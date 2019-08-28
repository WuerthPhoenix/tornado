pub mod standard;
pub mod tokio_actix;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // with tokio/actix
    tokio_actix::start_actix_stdin()

    // with std
    //standard::start_standard_stdin();
}
