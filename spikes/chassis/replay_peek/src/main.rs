//! Throwaway: print suspension travel of one vehicle of a replay over a time window. `replay_peek FILE VEHICLE T0 T1`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = w5k_replay::read_bin(std::path::Path::new(&a[1])).unwrap();
    let vi: usize = r.header.vehicles.iter().position(|v| v.name == a[2]).unwrap();
    let names = &r.header.vehicles[vi].joint_names;
    let idx: Vec<usize> = names.iter().enumerate().filter(|(_, n)| n.ends_with(".travel")).map(|(i, _)| i).collect();
    let (t0, t1): (f64, f64) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let mut maxv = vec![f32::MIN; idx.len()];
    for f in &r.frames {
        let Some(v) = f.vehicles.iter().find(|v| v.vehicle as usize == vi) else { continue };
        for (k, &i) in idx.iter().enumerate() {
            maxv[k] = maxv[k].max(v.joints[i]);
        }
        if f.t_s >= t0 && f.t_s <= t1 && ((f.t_s * 30.0).round() as i64) % 3 == 0 {
            let tr: Vec<String> = idx.iter().map(|&i| format!("{:6.0}", v.joints[i] * 1000.0)).collect();
            println!("t {:6.2} travel mm {}", f.t_s, tr.join(" "));
        }
    }
    println!("max travel over the whole run (mm): {:?}", maxv.iter().map(|m| (m * 1000.0).round()).collect::<Vec<_>>());
}
