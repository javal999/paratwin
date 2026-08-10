use std::time::Instant;

fn main() {
    let mut sc = paratwin_engine::preset("pilot");
    sc.days = 1;
    sc.replications = 1;
    let t = Instant::now();
    let out = paratwin_engine::des::simulate(&sc, 42, true);
    println!(
        "1 day pilot: {:?}  orders/day={:.0} sla={:.3}",
        t.elapsed(),
        out.kpi.orders_total,
        out.kpi.sla_attainment
    );
    let t = Instant::now();
    let _ = paratwin_engine::des::simulate(&sc, 43, false);
    println!("1 day pilot (no frames): {:?}", t.elapsed());
}
