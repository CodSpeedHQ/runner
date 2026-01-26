use divan::Bencher;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use runner_shared::artifacts::{MemtrackEvent, MemtrackEventKind, MemtrackWriter};

fn main() {
    divan::main();
}

/// Generate N random memtrack events with a seeded RNG
fn generate_events(n: usize) -> Vec<MemtrackEvent> {
    let mut rng = StdRng::seed_from_u64(12345);
    let mut events = Vec::with_capacity(n);
    for _ in 0..n {
        let size = rng.gen_range(8..8192);
        let kind = match rng.gen_range(0..8) {
            0 => MemtrackEventKind::Malloc { size },
            1 => MemtrackEventKind::Free,
            2 => MemtrackEventKind::Realloc {
                old_addr: Some(rng.r#gen()),
                size,
            },
            3 => MemtrackEventKind::Calloc { size },
            4 => MemtrackEventKind::AlignedAlloc { size },
            5 => MemtrackEventKind::Mmap { size },
            6 => MemtrackEventKind::Munmap { size },
            7 => MemtrackEventKind::Brk { size },
            _ => unreachable!(),
        };

        events.push(MemtrackEvent {
            pid: rng.r#gen(),
            tid: rng.r#gen(),
            timestamp: rng.r#gen(),
            addr: rng.r#gen(),
            kind,
        });
    }

    events
}

#[divan::bench(args = [10_000, 100_000, 500_000, 1_000_000])]
fn write_events(bencher: Bencher, n: usize) {
    let events = generate_events(n);

    bencher.bench_local(|| {
        let mut output = Vec::new();
        let mut writer = MemtrackWriter::new(&mut output).unwrap();
        for event in &events {
            writer.write_event(event).unwrap();
        }
        writer.finish().unwrap();
    });
}
