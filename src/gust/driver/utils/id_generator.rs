use idgenerator::*;
use std::time::Instant;

pub fn set_up_options() -> Result<(), OptionError> {
    // Setup the option for the id generator instance.
    let options = IdGeneratorOptions::new().worker_id(1).worker_id_bit_len(6);

    // Initialize the id generator instance with the option.
    // Other options not set will be given the default value.
    let _ = IdInstance::init(options)?;

    // Get the option from the id generator instance.
    let options = IdInstance::get_options();
    println!("First setting: {:?}", options);
    Ok(())
}

pub fn generate_id() -> i64 {
    let mut new_id: i64 = 0;
    let mut times = 100;
    let start = Instant::now();
    while times > 0 {
        // Call `next_id` to generate a new unique id.
        new_id = IdInstance::next_id();
        times -= 1;
    }
    let duration = start.elapsed().as_millis();
    println!(
        "Program finished after {} millis seconds! Last id {}",
        duration, new_id
    );
    new_id
}
