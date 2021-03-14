use manymouser::{Context, DriverOptions};
fn main() {
    let mut context = Context::new(&[DriverOptions::ManyMouse, DriverOptions::LinuxEvDev]).unwrap();
    println!("Time to get data!");
    println!("Driver: {}", context.driver_name());
    context
        .get_all_mouse_names()
        .for_each(|v| println!("mouse: {}", v.to_str().unwrap()));
    //let mouse_ids: Vec<_> = context.get_all_mouse_ids().collect();
    loop {
        let event = context.poll();
        if let Some(x) = event {
            println!("Name {}", x.device_id);
            println!("event: {:?}", x);
        }
    }
}
