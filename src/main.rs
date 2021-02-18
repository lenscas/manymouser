use manymouser::EventContext;
fn main() {
    let mut context = EventContext::new();
    println!("Time to get data!");
    println!("Driver: {}", context.driver_name());
    context.get_all_mouse_names().for_each(|v| {
        println!(
            "mouse: {}",
            String::from_utf8(v.iter().copied().collect()).unwrap()
        )
    });
    //let mouse_ids: Vec<_> = context.get_all_mouse_ids().collect();
    loop {
        let event = context.poll();
        if let Some(x) = event {
            println!("Name {}", x.device);
            println!("event: {:?}", x);
        }
    }
}
