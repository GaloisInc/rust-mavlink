#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate mavlink;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    // clone the data so it can be mutable
    let mut data = data.clone();
    // now attempt to parse the message
    // We expect either a valid message or an error
    // If we discover a panic!() or a segfault, we got a problem
    let _ = mavlink::read_v2_msg(&mut data);
});
