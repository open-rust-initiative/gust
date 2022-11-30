use super::id::ID;

struct Index{
    head: String,
    version: u32,
    number_of_objects: u32,
    signature: ID,
    fan_out_table:[u8;256],
    
}